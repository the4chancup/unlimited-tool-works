"""Acceptance-ID scanner: `acceptance.py report|strict` (CONTRIBUTING.md "Testing").

Scenarios are the lines of a plan's "Acceptance" section (any heading starting with that word,
under `docs/spec/` or `docs/plans/`) that begin, at column 0, with an ID followed by `GIVEN`, `manual  GIVEN`
or `withdrawn:`, or a range line, `TC-BIN-25 to 33  withdrawn:`, which defines each ID of the
run withdrawn. `docs/spec/withdrawn_ids.md`, the archive, holds the withdrawn lines that left
their spec, as written: each of its IDs is defined withdrawn, and every ID a range compacts must
be in it (a range line keeps no text of its own). A citation is a `//` comment made only of
IDs, directly above a `#[test]` line or another citation line, in any `.rs` file under
`crates/` (inline test modules included). A `manual` scenario is also proven by a worklog line `<ID> manual: checked <yyyy-mm-dd>`.

Both modes fail on what makes a citation or a definition untrustworthy: a citation of an ID no
scenario defines or of a withdrawn one, an ID-only comment not above a `#[test]`, a manual
check for a scenario not marked `manual`, an ID defined twice (an archived ID reused live
included), a line in an Acceptance section that starts with an ID but is not a scenario, a range
that does not run forward, an archive line that is not a withdrawn scenario, and a withdrawn line
both in its spec and archived. `report` (in `just gates`) lists unproven
scenarios, and the tests of a tool's CLI suite (`crates/tools/<tool>/tests/cli/`) that cite no
scenario, without failing, since an open phase's scenarios precede their tests; `strict` (at
converge) also fails on every unproven scenario and every uncited CLI test (unit tests cite
none: they check internals, which no scenario describes). Because Acceptance sections are written just in
time, every scenario in the plans and the spec at a converge belongs to the closing phase or an earlier one,
so strict checks them all.
"""

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

ID = r"[A-Z]{2}-[A-Z]+-\d+"
SCENARIO = re.compile(rf"^(?P<id>{ID})\s+(?:(?P<manual>manual)\s+)?(?P<kind>GIVEN\s|withdrawn:)")
RANGE = re.compile(r"^(?P<area>[A-Z]{2}-[A-Z]+-)(?P<first>\d+) to (?P<last>\d+)\s+withdrawn:")
STARTS_WITH_ID = re.compile(rf"^{ID}\b")
HEADING = re.compile(r"^(?P<level>#+)\s+(?P<title>.*)$")
CITATION = re.compile(rf"^\s*//\s*(?P<ids>{ID}(?:\s*,\s*{ID}|\s+{ID})*)\s*$")
CITED_ID = re.compile(ID)
MANUAL_CHECK = re.compile(rf"\b(?P<id>{ID}) manual: checked \d{{4}}-\d{{2}}-\d{{2}}\b")
CLI_SUITE = re.compile(r"^crates/tools/[^/]+/tests/cli/")
ARCHIVE = "docs/spec/withdrawn_ids.md"


@dataclass
class Scenario:
    manual: bool
    withdrawn: bool
    where: str
    ranged: bool = False


@dataclass
class Scan:
    scenarios: dict[str, Scenario] = field(default_factory=dict)
    archived: dict[str, str] = field(default_factory=dict)
    citations: dict[str, list[str]] = field(default_factory=dict)
    manual_checks: dict[str, list[str]] = field(default_factory=dict)
    uncited: list[str] = field(default_factory=list)
    problems: list[str] = field(default_factory=list)


def scan_plan(scan: Scan, name: str, text: str) -> None:
    section_level = 0
    in_fence = False
    for number, line in enumerate(text.splitlines(), start=1):
        if line.startswith("```"):
            in_fence = not in_fence
            continue
        heading = None if in_fence else HEADING.match(line)
        if heading:
            level = len(heading["level"])
            if heading["title"].startswith("Acceptance"):
                section_level = level
            elif section_level and level <= section_level:
                section_level = 0
            continue
        if not section_level or not STARTS_WITH_ID.match(line):
            continue
        where = f"{name}:{number}"
        ranged = RANGE.match(line)
        if ranged:
            first, last = int(ranged["first"]), int(ranged["last"])
            if first >= last:
                scan.problems.append(f"{where}: a range must run from a lower number to a higher one")
                continue
            width = len(ranged["first"])
            for n in range(first, last + 1):
                scenario = Scenario(manual=False, withdrawn=True, where=where, ranged=True)
                define(scan, f"{ranged['area']}{n:0{width}d}", scenario)
            continue
        scenario = SCENARIO.match(line)
        if not scenario:
            scan.problems.append(f"{where}: starts with an ID but is not a scenario line")
            continue
        define(
            scan,
            scenario["id"],
            Scenario(manual=scenario["manual"] is not None, withdrawn=scenario["kind"] == "withdrawn:", where=where),
        )


def define(scan: Scan, scenario_id: str, scenario: Scenario) -> None:
    if scenario_id in scan.scenarios:
        first = scan.scenarios[scenario_id].where
        scan.problems.append(f"{scenario.where}: {scenario_id} is already defined at {first}")
        return
    scan.scenarios[scenario_id] = scenario


def scan_archive(scan: Scan, name: str, text: str) -> None:
    for number, line in enumerate(text.splitlines(), start=1):
        starts = STARTS_WITH_ID.match(line)
        if not starts:
            continue
        where, scenario_id = f"{name}:{number}", starts[0]
        scenario = SCENARIO.match(line)
        if not scenario or scenario["kind"] != "withdrawn:":
            scan.problems.append(f"{where}: {scenario_id} is not a withdrawn scenario line")
        elif scenario_id in scan.archived:
            scan.problems.append(f"{where}: {scenario_id} is already defined at {scan.archived[scenario_id]}")
        else:
            scan.archived[scenario_id] = where


def check_archive(scan: Scan) -> None:
    """Reconciles the archive with the specs' definitions: an archived ID no spec holds is
    defined withdrawn at its archive line, a live scenario using one is a reuse, and every
    ID a range compacts must have its text archived, which no other line keeps."""
    for scenario_id, where in scan.archived.items():
        scenario = scan.scenarios.get(scenario_id)
        if scenario is None:
            scan.scenarios[scenario_id] = Scenario(manual=False, withdrawn=True, where=where)
        elif not scenario.withdrawn:
            scan.problems.append(f"{scenario.where}: {scenario_id} is already defined at {where}")
        elif not scenario.ranged:
            scan.problems.append(
                f"{where}: {scenario_id} is also written out at {scenario.where}; "
                "a withdrawn line stays or moves, not both"
            )
    for scenario_id, scenario in scan.scenarios.items():
        if scenario.ranged and scenario_id not in scan.archived:
            scan.problems.append(f"{scenario.where}: {scenario_id} is in a range but its text is not in {ARCHIVE}")


def scan_source(scan: Scan, name: str, text: str) -> None:
    cli_suite = CLI_SUITE.match(name) is not None
    pending: list[tuple[str, str]] = []
    for number, line in enumerate(text.splitlines(), start=1):
        citation = CITATION.match(line)
        if citation:
            where = f"{name}:{number}"
            pending.extend((cited, where) for cited in CITED_ID.findall(citation["ids"]))
            continue
        if line.strip() == "#[test]":
            if cli_suite and not pending:
                scan.uncited.append(f"{name}:{number}")
            for cited, where in pending:
                scan.citations.setdefault(cited, []).append(where)
        else:
            for cited, where in pending:
                scan.problems.append(f"{where}: citation of {cited} is not directly above a #[test]")
        pending = []
    for cited, where in pending:
        scan.problems.append(f"{where}: citation of {cited} is not directly above a #[test]")


def scan_worklog(scan: Scan, name: str, text: str) -> None:
    for number, line in enumerate(text.splitlines(), start=1):
        for check in MANUAL_CHECK.finditer(line):
            scan.manual_checks.setdefault(check["id"], []).append(f"{name}:{number}")


def reference_problem(scenario: Scenario | None, manual_only: bool) -> str | None:
    if scenario is None:
        return "defines no scenario"
    if scenario.withdrawn:
        return "is withdrawn"
    if manual_only and not scenario.manual:
        return "is not marked manual"
    return None


def check_references(scan: Scan) -> None:
    for kind, references, manual_only in (
        ("cites", scan.citations, False),
        ("manual check of", scan.manual_checks, True),
    ):
        for referenced, places in sorted(references.items()):
            problem = reference_problem(scan.scenarios.get(referenced), manual_only)
            if problem:
                scan.problems.extend(f"{where}: {kind} {referenced}, which {problem}" for where in places)


def unproven(scan: Scan) -> list[str]:
    return [
        scenario_id
        for scenario_id, scenario in scan.scenarios.items()
        if not scenario.withdrawn
        and scenario_id not in scan.citations
        and not (scenario.manual and scenario_id in scan.manual_checks)
    ]


def fails(scan: Scan, strict: bool) -> bool:
    return bool(scan.problems) or (strict and bool(unproven(scan) or scan.uncited))


def relative(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def scan_repository() -> Scan:
    scan = Scan()
    for tree in ("spec", "plans"):
        for path in sorted((ROOT / "docs" / tree).rglob("*.md")):
            scan_plan(scan, relative(path), path.read_text(encoding="utf-8"))
    scan_archive(scan, ARCHIVE, (ROOT / ARCHIVE).read_text(encoding="utf-8"))
    check_archive(scan)
    for path in sorted((ROOT / "crates").rglob("*.rs")):
        scan_source(scan, relative(path), path.read_text(encoding="utf-8"))
    worklog = ROOT / "docs" / "WORKLOG.md"
    scan_worklog(scan, relative(worklog), worklog.read_text(encoding="utf-8"))
    check_references(scan)
    return scan


def main(argv: list[str]) -> int:
    if len(argv) != 1 or argv[0] not in ("report", "strict"):
        print("usage: acceptance.py report|strict")
        return 2
    strict = argv[0] == "strict"
    scan = scan_repository()
    for problem in scan.problems:
        print(f"acceptance: {problem}")
    missing = unproven(scan)
    live = [scenario for scenario in scan.scenarios.values() if not scenario.withdrawn]
    manual = sum(scenario.manual for scenario in live)
    withdrawn = len(scan.scenarios) - len(live)
    print(
        f"acceptance: {len(live)} scenarios ({manual} manual, {withdrawn} withdrawn), "
        f"{len(live) - len(missing)} proven, {len(missing)} unproven"
    )
    if missing:
        print(f"acceptance: unproven: {', '.join(missing)}")
    if scan.uncited:
        print(f"acceptance: {len(scan.uncited)} CLI tests cite no scenario:")
        for where in scan.uncited:
            print(f"acceptance:   {where}")
    return 1 if fails(scan, strict) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
