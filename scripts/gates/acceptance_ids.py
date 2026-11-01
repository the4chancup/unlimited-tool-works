"""Print, for each scenario ID prefix or exact ID given, its definition, citations and manual checks.

Read-only: imports scripts/gates/acceptance.py and calls its scan.
Usage: python scripts/gates/acceptance_ids.py TC-SAV SE-STR TC-BIN-25 ...
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "gates"))

import acceptance  # noqa: E402


def main(args: list[str]) -> None:
    scan = acceptance.scan_repository()
    wanted = []
    for arg in args:
        if arg in scan.scenarios:
            wanted.append(arg)
        else:
            wanted.extend(sorted(i for i in scan.scenarios if i.startswith(arg + "-")))
    for scenario_id in wanted:
        scenario = scan.scenarios[scenario_id]
        flags = []
        if scenario.manual:
            flags.append("manual")
        if scenario.withdrawn:
            flags.append("withdrawn")
        cites = scan.citations.get(scenario_id, [])
        checks = scan.manual_checks.get(scenario_id, [])
        print(f"{scenario_id} [{','.join(flags)}] def {scenario.where}")
        for where in cites:
            print(f"    cite {where}")
        for where in checks:
            print(f"    manual {where}")
        if not cites and not checks and not scenario.withdrawn:
            print("    UNPROVEN")
    missing = [a for a in args if a not in scan.scenarios and not any(i.startswith(a + "-") for i in scan.scenarios)]
    for arg in missing:
        print(f"{arg}: no scenario defined")
    if not args:
        print("problems:", scan.problems)
        print("unproven:", acceptance.unproven(scan))
        print("uncited:", scan.uncited)


if __name__ == "__main__":
    main(sys.argv[1:])
