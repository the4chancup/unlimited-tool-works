"""Tests of scripts/gates/acceptance.py; `just acceptance` runs them before the scan."""

import unittest

from acceptance import (
    ARCHIVE,
    Scan,
    check_archive,
    check_references,
    fails,
    scan_archive,
    scan_plan,
    scan_source,
    scan_worklog,
    unproven,
)

PLAN = """# Tool

TC-X-99  GIVEN outside any Acceptance section

## Acceptance

Prose mentioning TC-KIT-01 mid-line is not a scenario.

```
TC-KIT-01  GIVEN a kit
           WHEN it is compiled
           THEN it is written
TC-KIT-02  withdrawn: replaced by TC-KIT-03
TC-GUI-01  manual  GIVEN the binary
                   WHEN it is launched
                   THEN a window opens
# a comment inside a fence is not a heading
TC-KIT-03  GIVEN a kit after a fenced comment
```

### Subsection

```
TC-KIT-04  GIVEN a kit in a subsection
```

## Next section

TC-X-98  GIVEN after the Acceptance section
"""


def scan_of(
    plan: str = PLAN, sources: dict[str, str] | None = None, worklog: str = "", archive: str = ""
) -> Scan:
    scan = Scan()
    scan_plan(scan, "plan.md", plan)
    scan_archive(scan, ARCHIVE, archive)
    check_archive(scan)
    for name, text in (sources or {}).items():
        scan_source(scan, name, text)
    scan_worklog(scan, "WORKLOG.md", worklog)
    check_references(scan)
    return scan


class PlanTests(unittest.TestCase):
    def test_reads_the_scenarios_of_acceptance_sections_only(self):
        scan = scan_of()
        self.assertEqual(sorted(scan.scenarios), ["TC-GUI-01", "TC-KIT-01", "TC-KIT-02", "TC-KIT-03", "TC-KIT-04"])
        self.assertEqual(scan.problems, [])

    def test_marks_manual_and_withdrawn_scenarios(self):
        scan = scan_of()
        self.assertTrue(scan.scenarios["TC-GUI-01"].manual)
        self.assertFalse(scan.scenarios["TC-KIT-01"].manual)
        self.assertTrue(scan.scenarios["TC-KIT-02"].withdrawn)
        self.assertFalse(scan.scenarios["TC-KIT-01"].withdrawn)
        self.assertEqual(scan.scenarios["TC-KIT-01"].where, "plan.md:10")

    def test_a_second_definition_of_an_id_is_a_problem(self):
        scan = scan_of("## Acceptance\nTC-A-01  GIVEN one\nTC-A-01  GIVEN two\n")
        self.assertEqual(scan.problems, ["plan.md:3: TC-A-01 is already defined at plan.md:2"])
        self.assertEqual(scan.scenarios["TC-A-01"].where, "plan.md:2")

    def test_an_id_line_that_is_not_a_scenario_is_a_problem(self):
        scan = scan_of("## Acceptance\nTC-A-01  GIVNE a typo\nTC-A-02 manual GIVEN fine\n")
        self.assertEqual(scan.problems, ["plan.md:2: starts with an ID but is not a scenario line"])
        self.assertEqual(sorted(scan.scenarios), ["TC-A-02"])


class CitationTests(unittest.TestCase):
    def test_a_citation_above_a_test_proves_its_scenarios(self):
        source = "mod tests {\n    // TC-KIT-01, TC-KIT-03\n    // TC-KIT-04\n    #[test]\n    fn kit() {}\n}\n"
        scan = scan_of(sources={"a.rs": source})
        self.assertEqual(scan.problems, [])
        self.assertEqual(scan.citations["TC-KIT-01"], ["a.rs:2"])
        self.assertEqual(scan.citations["TC-KIT-04"], ["a.rs:3"])
        self.assertEqual(unproven(scan), ["TC-GUI-01"])

    def test_a_citation_not_above_a_test_is_a_problem_and_proves_nothing(self):
        sources = {
            "a.rs": "// TC-KIT-01\n\n#[test]\nfn a() {}\n",
            "b.rs": "// TC-KIT-03\nfn helper() {}\n",
            "c.rs": "fn last() {}\n// TC-KIT-04\n",
        }
        scan = scan_of(sources=sources)
        self.assertEqual(
            scan.problems,
            [
                "a.rs:1: citation of TC-KIT-01 is not directly above a #[test]",
                "b.rs:1: citation of TC-KIT-03 is not directly above a #[test]",
                "c.rs:2: citation of TC-KIT-04 is not directly above a #[test]",
            ],
        )
        self.assertEqual(scan.citations, {})

    def test_comments_that_are_not_only_ids_are_not_citations(self):
        source = "// Like TC-KIT-01, but empty\n/// TC-KIT-03\n//! TC-KIT-04\n#[test]\nfn a() {}\n"
        scan = scan_of(sources={"a.rs": source})
        self.assertEqual(scan.problems, [])
        self.assertEqual(scan.citations, {})

    def test_a_cli_test_citing_no_scenario_is_listed_and_no_other_test_is(self):
        sources = {
            "crates/tools/tc/tests/cli/a.rs": "#[test]\nfn a() {}\n// TC-KIT-01\n#[test]\nfn b() {}\n",
            "crates/tools/tc/tests/cli/deep/b.rs": "fn helper() {}\n#[test]\nfn c() {}\n",
            "crates/tools/tc/tests/other.rs": "#[test]\nfn d() {}\n",
            "crates/tools/tc/src/x.rs": "#[test]\nfn unit() {}\n",
            "crates/libs/cli/tests/cli/y.rs": "#[test]\nfn lib() {}\n",
        }
        scan = scan_of(sources=sources)
        self.assertEqual(scan.problems, [])
        self.assertEqual(
            scan.uncited, ["crates/tools/tc/tests/cli/a.rs:1", "crates/tools/tc/tests/cli/deep/b.rs:2"]
        )

    def test_citing_an_undefined_or_withdrawn_scenario_is_a_problem(self):
        source = "// TC-KIT-02\n#[test]\nfn a() {}\n// TC-NOPE-01\n#[test]\nfn b() {}\n"
        scan = scan_of(sources={"a.rs": source})
        self.assertEqual(
            scan.problems,
            [
                "a.rs:1: cites TC-KIT-02, which is withdrawn",
                "a.rs:4: cites TC-NOPE-01, which defines no scenario",
            ],
        )


class ManualCheckTests(unittest.TestCase):
    def test_a_recorded_check_proves_a_manual_scenario(self):
        scan = scan_of(worklog="- 3.y converge\n  TC-GUI-01 manual: checked 2026-11-02, compiled from the GUI\n")
        self.assertEqual(scan.problems, [])
        self.assertIn("TC-KIT-01", unproven(scan))
        self.assertNotIn("TC-GUI-01", unproven(scan))

    def test_a_check_without_a_date_is_not_a_proof(self):
        scan = scan_of(worklog="TC-GUI-01 manual: checked soon\n")
        self.assertIn("TC-GUI-01", unproven(scan))

    def test_a_manual_check_of_an_automated_scenario_is_a_problem_and_proves_nothing(self):
        scan = scan_of(worklog="TC-KIT-01 manual: checked 2026-11-02, looked at it\n")
        self.assertEqual(scan.problems, ["WORKLOG.md:1: manual check of TC-KIT-01, which is not marked manual"])
        self.assertIn("TC-KIT-01", unproven(scan))

    def test_withdrawn_scenarios_are_never_unproven(self):
        self.assertNotIn("TC-KIT-02", unproven(scan_of()))


class WithdrawnArchiveTests(unittest.TestCase):
    PLAN = "## Acceptance\n```\nTC-A-01  GIVEN one\nTC-A-08 to 11  withdrawn: restated (each in the archive)\n```\n"
    ARCHIVED = (
        "# Withdrawn\n\nProse naming TC-A-08 mid-line.\n\n```\n"
        "TC-A-08  withdrawn: restated as TC-A-01\n"
        "         over two lines\n"
        "TC-A-09  withdrawn: restated as TC-A-01\n"
        "TC-A-10  withdrawn: restated as TC-A-01\n"
        "TC-A-11  withdrawn: restated as TC-A-01\n"
        "TC-OLD-01  withdrawn: the whole area went\n"
        "```\n"
    )

    def test_a_range_defines_each_of_its_ids_withdrawn_across_a_width_change(self):
        scan = scan_of(self.PLAN, archive=self.ARCHIVED)
        self.assertEqual(scan.problems, [])
        for scenario_id in ("TC-A-08", "TC-A-09", "TC-A-10", "TC-A-11"):
            self.assertTrue(scan.scenarios[scenario_id].withdrawn, scenario_id)
            self.assertEqual(scan.scenarios[scenario_id].where, "plan.md:4")
        self.assertNotIn("TC-A-12", scan.scenarios)
        self.assertNotIn("TC-A-07", scan.scenarios)

    def test_an_archived_id_no_spec_holds_is_defined_withdrawn_at_its_archive_line(self):
        scan = scan_of(self.PLAN, archive=self.ARCHIVED)
        self.assertTrue(scan.scenarios["TC-OLD-01"].withdrawn)
        self.assertEqual(scan.scenarios["TC-OLD-01"].where, f"{ARCHIVE}:11")

    def test_citing_or_checking_a_ranged_or_archived_id_is_a_problem(self):
        sources = {"a.rs": "// TC-A-10\n#[test]\nfn a() {}\n// TC-OLD-01\n#[test]\nfn b() {}\n"}
        scan = scan_of(self.PLAN, sources=sources, worklog="TC-A-09 manual: checked 2026-11-09\n", archive=self.ARCHIVED)
        self.assertEqual(
            scan.problems,
            [
                "a.rs:1: cites TC-A-10, which is withdrawn",
                "a.rs:4: cites TC-OLD-01, which is withdrawn",
                "WORKLOG.md:1: manual check of TC-A-09, which is withdrawn",
            ],
        )

    def test_ranged_and_archived_ids_are_never_unproven(self):
        scan = scan_of(self.PLAN, archive=self.ARCHIVED)
        self.assertLessEqual({"TC-A-10", "TC-OLD-01"}, scan.scenarios.keys())
        self.assertEqual(unproven(scan), ["TC-A-01"])

    def test_a_ranged_id_missing_from_the_archive_is_a_problem(self):
        archive = self.ARCHIVED.replace("TC-A-10  withdrawn: restated as TC-A-01\n", "")
        scan = scan_of(self.PLAN, archive=archive)
        self.assertEqual(scan.problems, [f"plan.md:4: TC-A-10 is in a range but its text is not in {ARCHIVE}"])

    def test_reusing_an_archived_id_for_a_live_scenario_is_a_problem(self):
        plan = "## Acceptance\n```\nTC-A-01  GIVEN one\nTC-A-08 to 11  withdrawn: restated\nTC-OLD-01  GIVEN a reused number\n```\n"
        scan = scan_of(plan, archive=self.ARCHIVED)
        self.assertEqual(scan.problems, [f"plan.md:5: TC-OLD-01 is already defined at {ARCHIVE}:11"])
        self.assertTrue(fails(scan, strict=False))

    def test_a_range_over_a_defined_id_is_a_problem(self):
        plan = "## Acceptance\n```\nTC-A-09  GIVEN live\nTC-A-08 to 11  withdrawn: restated\n```\n"
        scan = scan_of(plan, archive=self.ARCHIVED)
        self.assertEqual(
            scan.problems,
            ["plan.md:4: TC-A-09 is already defined at plan.md:3", f"plan.md:3: TC-A-09 is already defined at {ARCHIVE}:8"],
        )

    def test_a_range_that_does_not_run_forward_is_a_problem(self):
        scan = scan_of("## Acceptance\nTC-A-11 to 11  withdrawn: one\nTC-A-11 to 08  withdrawn: backwards\n")
        self.assertEqual(
            scan.problems,
            [
                "plan.md:2: a range must run from a lower number to a higher one",
                "plan.md:3: a range must run from a lower number to a higher one",
            ],
        )
        self.assertEqual(scan.scenarios, {})

    def test_a_withdrawn_line_both_in_place_and_archived_is_a_problem(self):
        plan = "## Acceptance\nTC-A-01  GIVEN one\nTC-OLD-01  withdrawn: the whole area went\n"
        scan = scan_of(plan, archive=self.ARCHIVED.replace("TC-A-", "TC-B-"))
        self.assertEqual(
            scan.problems,
            [f"{ARCHIVE}:11: TC-OLD-01 is also written out at plan.md:3; a withdrawn line stays or moves, not both"],
        )

    def test_an_archive_line_that_is_live_or_not_a_scenario_or_repeated_is_a_problem(self):
        archive = "```\nTC-C-01  GIVEN live in the archive\nTC-C-02 to 03  withdrawn: a range\nTC-C-04  withdrawn: once\nTC-C-04  withdrawn: twice\n```\n"
        scan = scan_of("", archive=archive)
        self.assertEqual(
            scan.problems,
            [
                f"{ARCHIVE}:2: TC-C-01 is not a withdrawn scenario line",
                f"{ARCHIVE}:3: TC-C-02 is not a withdrawn scenario line",
                f"{ARCHIVE}:5: TC-C-04 is already defined at {ARCHIVE}:4",
            ],
        )
        self.assertEqual(sorted(scan.scenarios), ["TC-C-04"])


class ModeTests(unittest.TestCase):
    PROVEN = "## Acceptance\nTC-A-01  GIVEN one\n"
    CITING = {"a.rs": "// TC-A-01\n#[test]\nfn a() {}\n"}

    def test_report_passes_unproven_scenarios_and_strict_fails_them(self):
        scan = scan_of(self.PROVEN)
        self.assertFalse(fails(scan, strict=False))
        self.assertTrue(fails(scan, strict=True))

    def test_both_modes_pass_when_every_scenario_is_proven(self):
        scan = scan_of(self.PROVEN, sources=self.CITING)
        self.assertFalse(fails(scan, strict=False))
        self.assertFalse(fails(scan, strict=True))

    def test_report_passes_an_uncited_cli_test_and_strict_fails_it(self):
        sources = {**self.CITING, "crates/tools/tc/tests/cli/a.rs": "#[test]\nfn b() {}\n"}
        scan = scan_of(self.PROVEN, sources=sources)
        self.assertFalse(fails(scan, strict=False))
        self.assertTrue(fails(scan, strict=True))

    def test_both_modes_fail_on_a_problem(self):
        scan = scan_of(self.PROVEN, sources={**self.CITING, "b.rs": "// TC-B-01\n#[test]\nfn b() {}\n"})
        self.assertTrue(fails(scan, strict=False))
        self.assertTrue(fails(scan, strict=True))


if __name__ == "__main__":
    unittest.main()
