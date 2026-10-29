"""Tests of scripts/run_tests.py's selection; `just test` runs them before the tests."""

import json
import unittest

from run_tests import ROOT, binaries, dependents, owner, packages, select


def meta(*crates: tuple[str, str, list[str]]) -> dict:
    """`cargo metadata` output for crates given as (name, folder under ROOT, dependencies)."""
    return {"packages": [
        {"name": name, "manifest_path": str(ROOT / folder / "Cargo.toml"),
         "dependencies": [{"name": d} for d in deps] + [{"name": "serde"}]}
        for name, folder, deps in crates
    ]}


# A workspace shaped like this one: a format lib, a lib on it, a tool, the binary on all.
PKGS = packages(meta(
    ("pes_version", "crates/libs/pes_version", []),
    ("pes_savefile", "crates/libs/pes_savefile", ["pes_version"]),
    ("save_editor", "crates/tools/save_editor", ["pes_savefile"]),
    ("ruleset_editor", "crates/tools/ruleset_editor", ["pes_version"]),
    ("studio", "crates/studio", ["save_editor", "ruleset_editor"]),
))


class Packages(unittest.TestCase):
    def test_only_workspace_dependencies_are_kept(self) -> None:
        self.assertEqual(PKGS["pes_savefile"].deps, {"pes_version"})

    def test_a_file_belongs_to_the_package_whose_folder_holds_it(self) -> None:
        path = ROOT / "crates/tools/save_editor/tests/fixtures/x.bin"
        self.assertEqual(owner(path, PKGS), "save_editor")
        self.assertIsNone(owner(ROOT / "resources/fonts/a.ttf", PKGS))

    def test_a_nested_package_wins_over_its_parent_folder(self) -> None:
        nested = packages(meta(("outer", "crates/outer", []), ("inner", "crates/outer/inner", [])))
        self.assertEqual(owner(ROOT / "crates/outer/inner/src/lib.rs", nested), "inner")
        self.assertEqual(owner(ROOT / "crates/outer/src/lib.rs", nested), "outer")

    def test_dependents_follow_the_graph_up_to_the_binary(self) -> None:
        self.assertEqual(dependents({"pes_savefile"}, PKGS),
                         {"pes_savefile", "save_editor", "studio"})
        self.assertEqual(dependents({"studio"}, PKGS), {"studio"})

    def test_dependents_do_not_rely_on_the_metadata_order(self) -> None:
        # `cargo metadata` lists packages by name, not dependencies first: the binary
        # before the tool before the lib.
        top_down = packages(meta(
            ("a_binary", "crates/a_binary", ["b_tool"]),
            ("b_tool", "crates/tools/b_tool", ["c_lib"]),
            ("c_lib", "crates/libs/c_lib", []),
        ))
        self.assertEqual(dependents({"c_lib"}, top_down), {"a_binary", "b_tool", "c_lib"})


class Select(unittest.TestCase):
    def test_a_tool_change_runs_the_tool_and_the_binary(self) -> None:
        changed = [ROOT / "crates/tools/save_editor/src/view/tactics.rs"]
        self.assertEqual(select(changed, PKGS), {"save_editor", "studio"})

    def test_a_lib_change_runs_everything_above_it(self) -> None:
        changed = [ROOT / "crates/libs/pes_version/src/lib.rs"]
        self.assertEqual(select(changed, PKGS), set(PKGS))

    def test_docs_alone_run_no_package(self) -> None:
        changed = [ROOT / "docs/WORKLOG.md", ROOT / "docs/plans/core/gui.md", ROOT / "README.md"]
        self.assertEqual(select(changed, PKGS), set())

    def test_the_mutation_exclusions_run_no_package(self) -> None:
        changed = [ROOT / ".cargo/mutants.toml", ROOT / ".cargo/extra_mutants.toml"]
        self.assertEqual(select(changed, PKGS), set())

    def test_a_script_no_test_run_goes_through_runs_no_package(self) -> None:
        changed = [ROOT / "scripts/release.py", ROOT / "scripts/mutants.py",
                   ROOT / "scripts/provenance/fixtures/interchange_fixtures.py"]
        self.assertEqual(select(changed, PKGS), set())

    def test_a_file_outside_every_package_runs_them_all(self) -> None:
        for outside in ["Cargo.lock", "rust-toolchain.toml", ".cargo/config.toml",
                        "resources/fonts/a.ttf", "scripts/test_runner.py",
                        "scripts/test_runner.cmd", "scripts/run_tests.py", "justfile"]:
            with self.subTest(outside):
                changed = [ROOT / "crates/tools/save_editor/src/lib.rs", ROOT / outside]
                self.assertEqual(select(changed, PKGS), set(PKGS))

    def test_a_markdown_file_inside_a_package_is_that_packages(self) -> None:
        # A crate's README or help chapter can be read by its tests (`include_str!`).
        changed = [ROOT / "crates/tools/ruleset_editor/help/01.md"]
        self.assertEqual(select(changed, PKGS), {"ruleset_editor", "studio"})


class Binaries(unittest.TestCase):
    def test_test_executables_are_taken_and_named_by_package(self) -> None:
        def artifact(folder: str, target: str, test: bool, exe: str | None) -> str:
            return json.dumps({
                "reason": "compiler-artifact",
                "manifest_path": str(ROOT / folder / "Cargo.toml"),
                "target": {"name": target},
                "profile": {"test": test},
                "executable": exe,
            })
        lines = "\n".join([
            artifact("crates/tools/save_editor", "save_editor", True, "a.exe"),
            artifact("crates/tools/save_editor", "cli", True, "b.exe"),
            artifact("crates/studio", "utw", False, "studio.exe"),
            artifact("crates/libs/pes_version", "pes_version", True, None),
            '{"reason": "build-finished", "success": true}',
            "Compiling a line that is not JSON",
        ])
        found = binaries(lines, PKGS)
        self.assertEqual([(b.key, b.exe) for b in found],
                         [("save_editor/save_editor", "a.exe"), ("save_editor/cli", "b.exe")])


if __name__ == "__main__":
    unittest.main()
