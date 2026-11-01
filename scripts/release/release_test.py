"""Tests of scripts/release/release.py; `just release` runs them before the recipe."""

import re
import tempfile
import tomllib
import unittest
from pathlib import Path

from release import (
    EMPTY_FOLDERS,
    LAUNCHERS,
    PACKAGE,
    about_config,
    existing_glob,
    expected_entries,
    exe_name,
    find_7z,
    launcher_calls,
    leaked_prefixes,
    links_crt_dynamically,
    pack,
    readme_names_version,
    third_party_notices,
    remap_flags,
    rustflags_config,
    sha256_line,
    symbols_entries,
    workspace_version,
)

ROOT = Path(__file__).resolve().parents[2]

HELP = """Unlimited Tool Works: the 4cc community's PES tools

Usage: UTW [OPTIONS] [COMMAND]

Commands:
  help  Print a help chapter, or export them all as Markdown
"""


class Pure(unittest.TestCase):
    def test_the_notices_accept_what_cargo_deny_allows(self) -> None:
        deny = (ROOT / "deny.toml").read_text(encoding="utf-8")
        config = tomllib.loads(about_config(deny))
        self.assertEqual(config["accepted"], tomllib.loads(deny)["licenses"]["allow"])
        self.assertIn("OFL-1.1", config["accepted"])
        self.assertEqual(config["targets"], ["x86_64-pc-windows-msvc"])

    def test_the_notices_list_each_license_once_with_every_crates_copyright(self) -> None:
        mit = "MIT License\n\n{}\n\nPermission is hereby granted, free of charge, ...\n"

        def used_by(name: str, version: str, manifest: str) -> dict:
            return {"crate": {"name": name, "version": version, "manifest_path": manifest}}

        registry = "C:/cargo/registry/src/index.crates.io"
        about = {"licenses": [
            {"id": "MIT", "name": "MIT License",
             "text": mit.format("Copyright (c) 2018 Andrew Gallant"),
             "used_by": [used_by("memchr", "2.7.4", f"{registry}/memchr-2.7.4/Cargo.toml")]},
            {"id": "MIT", "name": "MIT License",
             "text": mit.format("Copyright (c) 2015 The Rust Project Developers"),
             "used_by": [used_by("bitflags", "2.9.1", f"{registry}/bitflags-2.9.1/Cargo.toml"),
                         used_by("log", "0.4.27", f"{registry}/log-0.4.27/Cargo.toml"),
                         # The workspace's own crates are covered by the repository's licenses.
                         used_by("studio_core", "0.0.1", "C:/repo/crates/studio_core/Cargo.toml")]},
            {"id": "Apache-2.0", "name": "Apache License 2.0",
             "text": "Apache License\nVersion 2.0, January 2004\n\nCopyright [yyyy] [name of copyright owner]\n",
             "used_by": [used_by("zopfli", "0.8.3", f"{registry}/zopfli-0.8.3/Cargo.toml")]},
            # One copy of the license: lines opening with "copyright" are its text, not a notice.
            {"id": "OFL-1.1", "name": "SIL Open Font License 1.1",
             "text": "SIL OPEN FONT LICENSE Version 1.1\n4) The name(s) of the "
                     "Copyright Holder(s) or the Author(s) of the Font\nCopyright Holder(s) "
                     "and the Author(s) or with their explicit written\npermission.\n",
             "used_by": [used_by("epaint_default_fonts", "0.37.0",
                                 f"{registry}/epaint_default_fonts-0.37.0/Cargo.toml")]},
            # A copy whose author left the template's placeholder in.
            {"id": "MIT", "name": "MIT License",
             "text": mit.format("Copyright (c) <year> <copyright holders>"),
             "used_by": [used_by("windows-sys", "0.61.2", f"{registry}/windows-sys-0.61.2/Cargo.toml")]},
        ]}
        text = third_party_notices(about, Path("C:/repo"))
        self.assertEqual(text.count("=" * 80 + "\nMIT License\n"), 1)
        self.assertEqual(text.count("Permission is hereby granted"), 1)
        self.assertIn("  memchr 2.7.4: Copyright (c) 2018 Andrew Gallant\n", text)
        self.assertIn("  bitflags 2.9.1: Copyright (c) 2015 The Rust Project Developers\n", text)
        self.assertIn("  zopfli 0.8.3\n", text)
        self.assertIn("  windows-sys 0.61.2\n", text)
        self.assertIn("  epaint_default_fonts 0.37.0\n", text)
        self.assertIn("\nCopyright Holder(s) and the Author(s) or with their explicit written\n",
                      text)
        self.assertNotIn("studio_core", text)
        # The license text keeps no crate's copyright, only the crates' three lines do, and the
        # Apache text keeps its placeholder.
        self.assertEqual(text.count("Copyright (c)"), 3)
        self.assertIn("Copyright [yyyy] [name of copyright owner]", text)
        self.assertLess(text.index("MIT License"), text.index("Apache License 2.0"))

    def test_workspace_version(self) -> None:
        text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
        declared = re.search(r'\[workspace\.package\][^[]*?\nversion = "([^"]+)"', text)
        self.assertIsNotNone(declared)
        self.assertEqual(workspace_version(text), declared.group(1))
        self.assertEqual(workspace_version('[workspace.package]\nversion = "9.9.9"\n'), "9.9.9")

    def test_exe_name(self) -> None:
        self.assertEqual(exe_name(HELP), "UTW")
        self.assertIsNone(exe_name("no usage line\n"))

    def test_launcher_calls(self) -> None:
        for bat in ("quick_compile.bat", "quick_upgrade.bat"):
            text = (ROOT / "resources/bundle" / bat).read_text(encoding="utf-8")
            self.assertTrue(launcher_calls(text, "UTW"), bat)
            self.assertFalse(launcher_calls(text, "UnlimitedToolWorks"), bat)

    def test_readme_names_version(self) -> None:
        # The shipped readme against the workspace's version, which every release bumps with it.
        text = (ROOT / "resources/bundle/README.txt").read_text(encoding="utf-8")
        declared = workspace_version((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        self.assertTrue(readme_names_version(text, declared))
        self.assertTrue(readme_names_version("Unlimited Tool Works 0.0.1\n===\n", "0.0.1"))
        self.assertFalse(readme_names_version("Unlimited Tool Works 0.0.1\n===\n", "0.0.2"))
        # The program's name is not the script's to check: any name before the version.
        self.assertTrue(readme_names_version("UTW 1.0.0\n", "1.0.0"))
        self.assertFalse(readme_names_version("UTW 1.0.0.1\n", "1.0.0"))

    def test_the_bundled_settings_name_a_builtin_ruleset(self) -> None:
        settings = tomllib.loads(
            (ROOT / "resources/bundle/data/settings.toml").read_text(encoding="utf-8")
        )
        # DBC7's, for DBC's window testers; a stem `aatf` does not build in would fall back
        # to the first built-in with a notice on every start.
        self.assertEqual(settings["save-editor"]["aatf_ruleset"], "dbc7")
        # PES 2017, DBC's game, so the window opens on the save its testers bring.
        self.assertEqual(settings["common"]["pes_version"], 17)
        source = (ROOT / "crates/libs/aatf/src/templates.rs").read_text(encoding="utf-8")
        blocks = source.split("const BLOCKS: &[(&str, &str)] = &[", 1)[1].split("];", 1)[0]
        stems = re.findall(r'^\s*\("([a-z0-9]+)",', blocks, re.MULTILINE)
        self.assertIn("autumn26", stems, "the stems are read from the table")
        self.assertIn(settings["save-editor"]["aatf_ruleset"], stems)
        self.assertIs(settings["team-compiler"]["multicpk_mode"], False)
        self.assertEqual(settings["team-compiler"]["cpk_name"], "4cc_99_test")

    def test_expected_entries(self) -> None:
        self.assertEqual(
            expected_entries("UTW", "0.0.1"),
            {
                "unlimited-tool-works-0.0.1",
                "unlimited-tool-works-0.0.1/UTW.exe",
                "unlimited-tool-works-0.0.1/README.txt",
                "unlimited-tool-works-0.0.1/licenses",
                "unlimited-tool-works-0.0.1/licenses/LICENSE-MIT",
                "unlimited-tool-works-0.0.1/licenses/LICENSE-APACHE",
                "unlimited-tool-works-0.0.1/licenses/THIRD-PARTY-NOTICES.txt",
                "unlimited-tool-works-0.0.1/data",
                "unlimited-tool-works-0.0.1/data/settings.toml",
            },
        )

    def test_the_bundle_ships_no_launcher_and_no_export_folder(self) -> None:
        # 0.0.1 is the Save editor's alone: a launcher put back by habit fails here, not
        # in a member's unpacked folder.
        self.assertEqual(LAUNCHERS, ())
        self.assertEqual(EMPTY_FOLDERS, ())
        entries = expected_entries("UTW", "0.0.1")
        self.assertEqual([entry for entry in entries if entry.endswith(".bat")], [])
        for folder in ("exports", "old_exports"):
            self.assertNotIn(f"unlimited-tool-works-0.0.1/{folder}", entries)

    def test_sha256_line(self) -> None:
        self.assertEqual(sha256_line("ab12cd", "a.7z"), "ab12cd  a.7z\n")

    def test_symbols_entries(self) -> None:
        self.assertEqual(
            symbols_entries("UTW"),
            {"UTW.exe", "utw.pdb"},
        )

    def test_existing_glob_covers_the_bundle_and_the_symbols(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            dist = Path(tmp)
            for file in (
                "Tiny-0.0.1-windows.7z",
                "Tiny-0.0.1-windows.7z.sha256",
                "Tiny-0.0.1-symbols.7z",
            ):
                (dist / file).write_bytes(b"x")
            (dist / "Tiny-0.0.2-windows.7z").write_bytes(b"x")
            self.assertEqual(
                sorted(dist.glob(existing_glob("0.0.1"))),
                sorted(
                    dist / file
                    for file in (
                        "Tiny-0.0.1-windows.7z",
                        "Tiny-0.0.1-windows.7z.sha256",
                        "Tiny-0.0.1-symbols.7z",
                    )
                ),
            )
            self.assertEqual(
                list(dist.glob(existing_glob("0.0.2"))),
                [dist / "Tiny-0.0.2-windows.7z"],
            )

    def test_remap_flags_put_the_home_folder_first(self) -> None:
        # rustc applies the last matching prefix: the home folder, which holds the default
        # cargo and rustup homes, must lose to every more specific one.
        self.assertEqual(
            remap_flags(
                r"C:\Users\Member",
                r"C:\Users\Member\cargo",
                r"D:\rustup",
                r"C:\Users\Member\src\utw",
                r"E:\build\utw",
            ),
            [
                r"--remap-path-prefix=C:\Users\Member=~",
                r"--remap-path-prefix=C:\Users\Member\cargo=~cargo",
                r"--remap-path-prefix=D:\rustup=~rustup",
                r"--remap-path-prefix=C:\Users\Member\src\utw=.",
                r"--remap-path-prefix=E:\build\utw=target",
            ],
        )
        # Unset homes and a target directory inside the repository add nothing.
        self.assertEqual(
            remap_flags(r"C:\Users\Member", None, None, r"C:\src\utw", r"C:\src\utw\target"),
            [r"--remap-path-prefix=C:\Users\Member=~", r"--remap-path-prefix=C:\src\utw=."],
        )

    def test_rustflags_config_is_the_toml_array_of_the_flags(self) -> None:
        flags = [r"--remap-path-prefix=C:\Users\O'Brien \"x\"=~", "--remap-path-prefix=C:\\src=."]
        parsed = tomllib.loads(rustflags_config(flags))
        self.assertEqual(parsed["target"]["x86_64-pc-windows-msvc"]["rustflags"], flags)

    def test_the_config_sets_crt_static_on_the_key_the_release_joins(self) -> None:
        # `--config` joins arrays on one key; on another key (`build.rustflags`) the target's
        # own array would win and the remap would be dropped, or the other way round.
        config = tomllib.loads((ROOT / ".cargo/config.toml").read_text(encoding="utf-8"))
        target_flags = config["target"]["x86_64-pc-windows-msvc"]["rustflags"]
        self.assertIn("target-feature=+crt-static", target_flags)
        self.assertNotIn("rustflags", config.get("build", {}))
        self.assertTrue(rustflags_config([]).startswith("target.x86_64-pc-windows-msvc.rustflags="))

    def test_leaked_prefixes_match_any_case_and_separator(self) -> None:
        exe = b"\x00panicked at c:/users/member\\.cargo\\registry\\src\\x.rs\x00./target/out.rs"
        self.assertEqual(
            leaked_prefixes(exe, [r"C:\Users\Member", r"C:\src\utw"]), [r"C:\Users\Member"]
        )
        self.assertEqual(leaked_prefixes(b"~\\.cargo\\registry\\src\\x.rs", [r"C:\Users\Member"]), [])

    def test_links_crt_dynamically(self) -> None:
        self.assertTrue(links_crt_dynamically(b"\x00VCRUNTIME140.dll\x00"))
        self.assertFalse(links_crt_dynamically(b"\x00KERNEL32.dll\x00"))


class Listing(unittest.TestCase):
    def test_a_packed_bundle_lists_the_staged_set(self) -> None:
        seven_z = find_7z()
        if seven_z is None:
            self.fail("7-Zip not found: neither `7z` on PATH nor C:\\Program Files\\7-Zip\\7z.exe")
        with tempfile.TemporaryDirectory() as tmp:
            stage = Path(tmp)
            top = stage / f"{PACKAGE}-9.9.9"
            (top / "data").mkdir(parents=True)
            (top / "data/settings.toml").write_bytes(b"x")
            (top / "licenses").mkdir()
            for bundled in (
                "Tiny.exe",
                "README.txt",
                "licenses/LICENSE-MIT",
                "licenses/LICENSE-APACHE",
                "licenses/THIRD-PARTY-NOTICES.txt",
            ):
                (top / bundled).write_bytes(b"x")
            found = pack(seven_z, stage / "tiny.7z", stage, [top.name])
            expected = expected_entries("Tiny", "9.9.9")
            self.assertEqual(found, expected)
            self.assertNotEqual(found, expected - {f"{PACKAGE}-9.9.9/README.txt"})

    def test_a_packed_symbols_archive_lists_its_two_entries(self) -> None:
        # Two bare names under the staging folder, as `release.py` packs the symbols: the
        # archive's entries are flat.
        seven_z = find_7z()
        if seven_z is None:
            self.fail("7-Zip not found: neither `7z` on PATH nor C:\\Program Files\\7-Zip\\7z.exe")
        with tempfile.TemporaryDirectory() as tmp:
            stage = Path(tmp)
            (stage / "Tiny.exe").write_bytes(b"x")
            (stage / "utw.pdb").write_bytes(b"y")
            found = pack(seven_z, stage / "tiny-symbols.7z", stage, ["Tiny.exe", "utw.pdb"])
            self.assertEqual(found, symbols_entries("Tiny"))


if __name__ == "__main__":
    unittest.main()
