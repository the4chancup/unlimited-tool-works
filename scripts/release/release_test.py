"""Tests of scripts/release/release.py; `just release` runs them before the recipe."""

import re
import sys
import tarfile
import tempfile
import tomllib
import unittest
from pathlib import Path

from release import (
    EMPTY_FOLDERS,
    LAUNCHERS,
    LICENSES,
    PACKAGE,
    Refusal,
    about_config,
    build_args,
    c_flags_env,
    check_exe,
    elf_sections,
    existing_glob,
    expected_entries,
    exe_name,
    find_7z,
    launcher_calls,
    leaked_prefixes,
    leftover_debug,
    links_crt_dynamically,
    linux_symbols_entries,
    pack,
    pack_tar,
    pack_tar_flat,
    platform_name,
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

READELF_UNSTRIPPED = """There are 36 section headers, starting at offset 0x1195ae8:

Section Headers:
  [Nr] Name              Type            Address          Off    Size   ES Flg Lk Inf Al
  [ 0]                   NULL            0000000000000000 000000 000000 00      0   0  0
  [ 1] .interp           PROGBITS        0000000000000318 000318 00001c 00   A  0   0  1
  [14] .text             PROGBITS        0000000000003d60 002d60 a2c4e1 00  AX  0   0 16
  [30] .debug_info       PROGBITS        0000000000000000 a35f00 5b7a2e 00      0   0  1
  [31] .debug_abbrev     PROGBITS        0000000000000000 fed92e 001f4a 00      0   0  1
  [33] .symtab           SYMTAB          0000000000000000 fee878 1b2a30 18     34 500 8
  [34] .strtab           STRTAB          0000000000000000 11a12a8 15b7c0 00      0   0  1
  [35] .shstrtab         STRTAB          0000000000000000 12fca68 00017f 00      0   0  1
Key to Flags:
  W (write), A (alloc), X (execute), M (merge), S (strings), I (info)
"""

READELF_STRIPPED = """There are 31 section headers, starting at offset 0xa2cfe8:

Section Headers:
  [Nr] Name              Type            Address          Off    Size   ES Flg Lk Inf Al
  [ 0]                   NULL            0000000000000000 000000 000000 00      0   0  0
  [14] .text             PROGBITS        0000000000003d60 002d60 a2c4e1 00  AX  0   0 16
  [17] .debug_gdb_scripts PROGBITS       0000000000a2c4f0 a2c4f0 000022 01 AMS  0   0  1
  [28] .gnu_debuglink    PROGBITS        0000000000000000 a2c560 000014 00      0   0  4
  [29] .symtab           SYMTAB          0000000000000000 a2c580 1b2a30 18     30 500 8
  [30] .strtab           STRTAB          0000000000000000 bdefb0 15b7c0 00      0   0  1
"""


class Pure(unittest.TestCase):
    def test_the_notices_accept_what_cargo_deny_allows(self) -> None:
        deny = (ROOT / "deny.toml").read_text(encoding="utf-8")
        config = tomllib.loads(about_config(deny, "x86_64-pc-windows-msvc"))
        self.assertEqual(config["accepted"], tomllib.loads(deny)["licenses"]["allow"])
        self.assertIn("OFL-1.1", config["accepted"])
        self.assertEqual(config["targets"], ["x86_64-pc-windows-msvc"])

    def test_the_notices_read_the_target_the_binary_builds_for(self) -> None:
        # The two platforms link different crates, so the Linux bundle's notices come
        # from the Linux target's own dependency graph.
        deny = (ROOT / "deny.toml").read_text(encoding="utf-8")
        config = tomllib.loads(about_config(deny, "x86_64-unknown-linux-gnu"))
        self.assertEqual(config["targets"], ["x86_64-unknown-linux-gnu"])

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

    def test_the_notices_refuse_a_graph_with_no_third_party_crate(self) -> None:
        # An empty licenses list or one whose every crate is the workspace's would
        # ship the header alone as the notices file.
        with self.assertRaisesRegex(Refusal, "cargo-about listed no third-party crate"):
            third_party_notices({"licenses": []}, Path("C:/repo"))
        about = {"licenses": [
            {"id": "MIT", "name": "MIT License", "text": "MIT License\n",
             "used_by": [{"crate": {"name": "studio_core", "version": "0.0.1",
                                    "manifest_path": "C:/repo/crates/studio_core/Cargo.toml"}}]},
        ]}
        with self.assertRaisesRegex(Refusal, "cargo-about listed no third-party crate"):
            third_party_notices(about, Path("C:/repo"))

    def test_a_license_clause_is_not_a_copyright_line(self) -> None:
        # One Apache-2.0 copy without clause 4's lettered conditions leaves them
        # unshared; they are license text, not a crate's notice.
        clause = "(c) You must retain, in the Source form of any Derivative Works"
        full = (
            "Apache License\n"
            "   (b) You must cause any modified files to carry prominent notices; and\n"
            f"   {clause}\n"
            "       that You distribute, all copyright, patent, trademark, and\n"
            "   (d) If the Work includes a NOTICE text file\n"
        )
        grant = "Apache License\n\nCopyright 2022 Kirill Chibisov\n\nLicensed under ...\n"

        def used_by(name: str, manifest: str) -> dict:
            return {"crate": {"name": name, "version": "1.0.0", "manifest_path": manifest}}

        about = {"licenses": [
            {"id": "Apache-2.0", "name": "Apache License 2.0", "text": full,
             "used_by": [used_by("accesskit_winit", "C:/reg/accesskit_winit-1.0.0/Cargo.toml")]},
            {"id": "Apache-2.0", "name": "Apache License 2.0", "text": grant,
             "used_by": [used_by("glutin_wgl_sys", "C:/reg/glutin_wgl_sys-1.0.0/Cargo.toml")]},
        ]}
        text = third_party_notices(about, Path("C:/repo"))
        crate_lines = [
            line for line in text.splitlines()
            if line.startswith("  ") and not line.startswith("   ")
        ]
        self.assertFalse(any("You must retain" in line for line in crate_lines), crate_lines)
        self.assertIn(f"   {clause}\n", text)
        self.assertIn("  glutin_wgl_sys 1.0.0: Copyright 2022 Kirill Chibisov\n", text)

    def test_an_unfilled_copyright_template_is_not_a_notice(self) -> None:
        # A `Copyright {yyyy} {name of copyright owner}` line is license text like the
        # appendix's `[yyyy]` and an MIT copy's `<year>`, not the crate's notice.
        mit = "MIT License\n\n{}\n\nPermission is hereby granted ...\n"

        def used_by(name: str, version: str) -> dict:
            return {"crate": {"name": name, "version": version,
                              "manifest_path": f"C:/reg/{name}-{version}/Cargo.toml"}}

        about = {"licenses": [
            {"id": "MIT", "name": "MIT License",
             "text": mit.format("Copyright {yyyy} {name of copyright owner}\n"
                                "Copyright 2024 dpi contributors"),
             "used_by": [used_by("dpi", "0.1.2")]},
            {"id": "MIT", "name": "MIT License",
             "text": mit.format("Copyright 2020 Someone Else"),
             "used_by": [used_by("memchr", "2.7.4")]},
        ]}
        text = third_party_notices(about, Path("C:/repo"))
        self.assertIn("  dpi 0.1.2: Copyright 2024 dpi contributors\n", text)

    def test_a_crate_in_two_copies_of_a_license_prints_once(self) -> None:
        # cargo-about lists a license once per distinct text; a crate in two copies'
        # used_by is still one crate line.
        def used_by() -> dict:
            return {"crate": {"name": "sevenz-rust2", "version": "0.22.2",
                              "manifest_path": "C:/reg/sevenz-rust2-0.22.2/Cargo.toml"}}

        about = {"licenses": [
            {"id": "Apache-2.0", "name": "Apache License 2.0", "text": "Apache License\n",
             "used_by": [used_by()]},
            {"id": "Apache-2.0", "name": "Apache License 2.0",
             "text": "Apache License\nApache License\n",
             "used_by": [used_by()]},
        ]}
        text = third_party_notices(about, Path("C:/repo"))
        self.assertEqual(text.count("  sevenz-rust2 0.22.2\n"), 1)

    def test_workspace_version(self) -> None:
        text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
        declared = re.search(r'\[workspace\.package\][^[]*?\nversion = "([^"]+)"', text)
        self.assertIsNotNone(declared)
        self.assertEqual(workspace_version(text), declared.group(1))
        self.assertEqual(workspace_version('[workspace.package]\nversion = "9.9.9"\n'), "9.9.9")

    def test_exe_name(self) -> None:
        self.assertEqual(exe_name(HELP), "UTW")
        self.assertIsNone(exe_name("no usage line\n"))

    def test_platform_name(self) -> None:
        self.assertEqual(platform_name("win32"), "windows")
        self.assertEqual(platform_name("linux"), "linux")
        with self.assertRaisesRegex(Refusal, "Windows and Linux only"):
            platform_name("darwin")

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
            expected_entries("UTW", "0.0.1", "windows"),
            {
                "unlimited-tool-works-0.0.1",
                "unlimited-tool-works-0.0.1/UTW.exe",
                "unlimited-tool-works-0.0.1/README.txt",
                "unlimited-tool-works-0.0.1/licenses",
                "unlimited-tool-works-0.0.1/licenses/LICENSE-MIT",
                "unlimited-tool-works-0.0.1/licenses/LICENSE-APACHE",
                "unlimited-tool-works-0.0.1/licenses/RobotoFlex-OFL.txt",
                "unlimited-tool-works-0.0.1/licenses/PHOSPHOR-LICENSE.txt",
                "unlimited-tool-works-0.0.1/licenses/THIRD-PARTY-NOTICES.txt",
                "unlimited-tool-works-0.0.1/data",
                "unlimited-tool-works-0.0.1/data/settings.toml",
            },
        )

    def test_expected_entries_linux(self) -> None:
        entries = expected_entries("utw", "0.0.1", "linux")
        self.assertIn("unlimited-tool-works-0.0.1/utw", entries)
        self.assertEqual([entry for entry in entries if entry.endswith(".exe")], [])

    def test_the_bundle_ships_the_bundled_works_licenses(self) -> None:
        # Roboto Flex's OFL and Phosphor's MIT, the third-party works the suite bundles
        # outside a crate, stage under `licenses/` beside the project's own two.
        entries = expected_entries("UTW", "0.0.1", "windows")
        self.assertIn("unlimited-tool-works-0.0.1/licenses/RobotoFlex-OFL.txt", entries)
        self.assertIn("unlimited-tool-works-0.0.1/licenses/PHOSPHOR-LICENSE.txt", entries)
        for license_file in LICENSES:
            self.assertTrue((ROOT / license_file).is_file(), license_file)

    def test_the_bundle_ships_no_launcher_and_no_export_folder(self) -> None:
        # 0.0.1 is the Save editor's alone: a launcher put back by habit fails here, not
        # in a member's unpacked folder.
        self.assertEqual(LAUNCHERS, ())
        self.assertEqual(EMPTY_FOLDERS, ())
        entries = expected_entries("UTW", "0.0.1", "windows")
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

    def test_linux_symbols_entries(self) -> None:
        self.assertEqual(linux_symbols_entries("utw"), {"utw.debug"})

    def test_existing_glob_covers_both_bundles_and_the_symbols(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            dist = Path(tmp)
            for file in (
                "Tiny-0.0.1-windows.7z",
                "Tiny-0.0.1-windows.7z.sha256",
                "Tiny-0.0.1-linux.tar.gz",
                "Tiny-0.0.1-linux.tar.gz.sha256",
                "Tiny-0.0.1-symbols.7z",
                "Tiny-0.0.1-linux-symbols.tar.gz",
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
                        "Tiny-0.0.1-linux.tar.gz",
                        "Tiny-0.0.1-linux.tar.gz.sha256",
                        "Tiny-0.0.1-symbols.7z",
                        "Tiny-0.0.1-linux-symbols.tar.gz",
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
        # Unset homes and a target directory inside the repository add nothing. The paths are
        # the host's own: "inside" is the host's path rule, so a Windows path under Linux is
        # inside nothing.
        home, root = str(Path("/home/member")), str(Path("/src/utw"))
        self.assertEqual(
            remap_flags(home, None, None, root, str(Path(root) / "target")),
            [f"--remap-path-prefix={home}=~", f"--remap-path-prefix={root}=."],
        )

    def test_rustflags_config_is_the_toml_array_of_the_flags(self) -> None:
        flags = [r"--remap-path-prefix=C:\Users\O'Brien \"x\"=~", "--remap-path-prefix=C:\\src=."]
        parsed = tomllib.loads(rustflags_config(flags, "x86_64-pc-windows-msvc"))
        self.assertEqual(parsed["target"]["x86_64-pc-windows-msvc"]["rustflags"], flags)
        parsed = tomllib.loads(rustflags_config(flags, "x86_64-unknown-linux-gnu"))
        self.assertEqual(parsed["target"]["x86_64-unknown-linux-gnu"]["rustflags"], flags)

    def test_the_config_sets_crt_static_on_the_key_the_release_joins(self) -> None:
        # `--config` joins arrays on one key; on another key (`build.rustflags`) the target's
        # own array would win and the remap would be dropped, or the other way round.
        config = tomllib.loads((ROOT / ".cargo/config.toml").read_text(encoding="utf-8"))
        target_flags = config["target"]["x86_64-pc-windows-msvc"]["rustflags"]
        self.assertIn("target-feature=+crt-static", target_flags)
        self.assertNotIn("rustflags", config.get("build", {}))
        self.assertTrue(
            rustflags_config([], "x86_64-pc-windows-msvc")
            .startswith("target.x86_64-pc-windows-msvc.rustflags=")
        )

    def test_build_args(self) -> None:
        # The one cargo build command the release and the candidate build share.
        self.assertEqual(
            build_args([r"--remap-path-prefix=C:\u=~"], "x86_64-pc-windows-msvc"),
            [
                "cargo",
                "build",
                "--release",
                "--locked",
                "-p",
                "studio",
                "--config",
                'target.x86_64-pc-windows-msvc.rustflags=["--remap-path-prefix=C:\\\\u=~"]',
            ],
        )
        self.assertEqual(
            build_args(["--remap-path-prefix=/home/runner=~"], "x86_64-unknown-linux-gnu"),
            [
                "cargo",
                "build",
                "--release",
                "--locked",
                "-p",
                "studio",
                "--config",
                'target.x86_64-unknown-linux-gnu.rustflags=["--remap-path-prefix=/home/runner=~"]',
            ],
        )

    def test_leaked_prefixes_match_any_case_and_separator(self) -> None:
        exe = b"\x00panicked at c:/users/member\\.cargo\\registry\\src\\x.rs\x00./target/out.rs"
        self.assertEqual(
            leaked_prefixes(exe, [r"C:\Users\Member", r"C:\src\utw"]), [r"C:\Users\Member"]
        )
        self.assertEqual(leaked_prefixes(b"~\\.cargo\\registry\\src\\x.rs", [r"C:\Users\Member"]), [])

    def test_links_crt_dynamically(self) -> None:
        self.assertTrue(links_crt_dynamically(b"\x00VCRUNTIME140.dll\x00"))
        self.assertFalse(links_crt_dynamically(b"\x00KERNEL32.dll\x00"))

    def test_elf_sections(self) -> None:
        # The check the recipe runs on the stripped Linux binary: no `.debug_*` left,
        # the symbol table kept (`.gnu_debuglink` is not a `.debug_` section, and the
        # unnamed first section's `NULL` is its type, not a name).
        self.assertEqual(
            elf_sections(READELF_UNSTRIPPED),
            {
                ".interp",
                ".text",
                ".debug_info",
                ".debug_abbrev",
                ".symtab",
                ".strtab",
                ".shstrtab",
            },
        )
        stripped = elf_sections(READELF_STRIPPED)
        self.assertEqual(
            stripped, {".text", ".debug_gdb_scripts", ".gnu_debuglink", ".symtab", ".strtab"}
        )

    def test_leftover_debug_passes_the_section_a_strip_keeps(self) -> None:
        # `.debug_gdb_scripts` is loaded (flag A), so `objcopy --strip-debug` keeps it: the
        # second dry run's stripped binary held it and was refused. Debug info proper is not.
        self.assertEqual(leftover_debug(elf_sections(READELF_STRIPPED)), [])
        self.assertEqual(
            leftover_debug(elf_sections(READELF_UNSTRIPPED)), [".debug_abbrev", ".debug_info"]
        )

    def test_check_exe(self) -> None:
        # The check `release()` runs on its build and the candidate runs on the exe it
        # finds: a leaked build-machine path or a dynamic C runtime refuses.
        with tempfile.TemporaryDirectory() as tmp:
            exe = Path(tmp) / "utw.exe"
            exe.write_bytes(b"\x00panicked at c:/users/member/.cargo/registry/src/x.rs\x00")
            with self.assertRaisesRegex(Refusal, r"C:\\Users\\Member"):
                check_exe(exe, [r"C:\Users\Member", r"C:\src\utw"], "windows")
            exe.write_bytes(b"\x00VCRUNTIME140.dll\x00")
            with self.assertRaisesRegex(Refusal, r"VCRUNTIME140\.dll"):
                check_exe(exe, [], "windows")
            exe.write_bytes(b"\x00a clean exe\x00")
            check_exe(exe, [r"C:\Users\Member", r"C:\src\utw"], "windows")

    def test_check_exe_linux(self) -> None:
        # A Linux binary links the system's libc, so naming a C runtime is no refusal;
        # a leaked build-machine path still is.
        with tempfile.TemporaryDirectory() as tmp:
            exe = Path(tmp) / "utw"
            exe.write_bytes(b"\x00vcruntime140.dll\x00")
            check_exe(exe, [], "linux")
            exe.write_bytes(b"\x00/home/runner/work/utw/x.rs\x00")
            with self.assertRaisesRegex(Refusal, "/home/runner"):
                check_exe(exe, ["/home/runner/work/utw"], "linux")

    def test_the_c_compiler_maps_the_same_prefixes_as_rustc(self) -> None:
        # C and assembly built through `cc` (ring's) keep their source paths in their debug
        # info, which rustc's remap never sees: on Linux the same pairs go to the C compiler,
        # in rustc's order (both apply the last matching prefix), after CFLAGS' own.
        flags = remap_flags("/home/m", "/home/m/.cargo", None, "/src/utw", "/src/utw/target")
        env = c_flags_env(flags, "linux", {"CFLAGS": "-O2"})
        # Each pair twice: gcc 11 hands only `-fdebug-prefix-map` on to the assembler, so
        # an assembly file's (ring's `.S`) paths need that spelling too.
        self.assertEqual(
            env["CFLAGS"],
            "-O2 -ffile-prefix-map=/home/m=~ -fdebug-prefix-map=/home/m=~"
            " -ffile-prefix-map=/home/m/.cargo=~cargo -fdebug-prefix-map=/home/m/.cargo=~cargo"
            " -ffile-prefix-map=/src/utw=. -fdebug-prefix-map=/src/utw=.",
        )
        self.assertEqual(env["CXXFLAGS"], env["CFLAGS"].removeprefix("-O2 "))
        # MSVC's cl takes no such flag; Windows keeps C debug info in the .pdb.
        self.assertEqual(c_flags_env(flags, "windows", {"CFLAGS": "-O2"}), {"CFLAGS": "-O2"})

    def test_a_refusal_names_the_strings_that_hold_a_path(self) -> None:
        # The refusal shows where a path sits, so a leak is diagnosed from the log alone;
        # each string once, the strings without it left out.
        with tempfile.TemporaryDirectory() as tmp:
            exe = Path(tmp) / "utw"
            exe.write_bytes(
                b"\x01\x02/home/runner/.cargo/registry/src/a.c\x00clean text\x00"
                b"\xff/home/runner/.cargo/registry/src/a.c\x00x/home/runner/work/b.S\x7f"
            )
            with self.assertRaises(Refusal) as refused:
                check_exe(exe, ["/home/runner"], "linux")
            message = str(refused.exception)
            self.assertIn("/home/runner/.cargo/registry/src/a.c", message)
            self.assertIn("x/home/runner/work/b.S", message)
            self.assertEqual(message.count("registry/src/a.c"), 1)
            self.assertNotIn("clean text", message)


class Listing(unittest.TestCase):
    @unittest.skipUnless(sys.platform == "win32", "7-Zip packs the Windows bundle only")
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
                "licenses/RobotoFlex-OFL.txt",
                "licenses/PHOSPHOR-LICENSE.txt",
                "licenses/THIRD-PARTY-NOTICES.txt",
            ):
                (top / bundled).write_bytes(b"x")
            found = pack(seven_z, stage / "tiny.7z", stage, [top.name])
            expected = expected_entries("Tiny", "9.9.9", "windows")
            self.assertEqual(found, expected)
            self.assertNotEqual(found, expected - {f"{PACKAGE}-9.9.9/README.txt"})

    @unittest.skipUnless(sys.platform == "win32", "7-Zip packs the Windows bundle only")
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

    def test_a_packed_tarball_lists_the_staged_set_and_marks_the_binary(self) -> None:
        # The Linux bundle: the same staged set as the .7z, but a tar keeps modes and
        # ownership, so the binary must be executable and no member may name an owner.
        with tempfile.TemporaryDirectory() as tmp:
            stage = Path(tmp)
            top = stage / f"{PACKAGE}-9.9.9"
            (top / "data").mkdir(parents=True)
            (top / "data/settings.toml").write_bytes(b"x")
            (top / "licenses").mkdir()
            for bundled in (
                "Tiny",
                "README.txt",
                "licenses/LICENSE-MIT",
                "licenses/LICENSE-APACHE",
                "licenses/RobotoFlex-OFL.txt",
                "licenses/PHOSPHOR-LICENSE.txt",
                "licenses/THIRD-PARTY-NOTICES.txt",
            ):
                (top / bundled).write_bytes(b"x")
            archive = stage / "tiny.tar.gz"
            found = pack_tar(archive, stage, top.name, "Tiny")
            self.assertEqual(found, expected_entries("Tiny", "9.9.9", "linux"))
            with tarfile.open(archive) as tar:
                members = {member.name: member for member in tar.getmembers()}
            self.assertEqual(members[top.name].mode, 0o755)
            self.assertEqual(members[f"{top.name}/Tiny"].mode, 0o755)
            self.assertEqual(members[f"{top.name}/README.txt"].mode, 0o644)
            self.assertEqual(members[f"{top.name}/data"].mode, 0o755)
            for member in members.values():
                self.assertEqual(
                    (member.uid, member.gid, member.uname, member.gname),
                    (0, 0, "", ""),
                    member.name,
                )

    def test_a_packed_flat_tarball_lists_the_debug_file(self) -> None:
        # The Linux symbols archive, packed as the recipe packs it: `utw.debug` alone
        # and flat, `0644`, owned by nobody, as the Windows symbols' entries are flat.
        with tempfile.TemporaryDirectory() as tmp:
            stage = Path(tmp)
            (stage / "utw.debug").write_bytes(b"x")
            archive = stage / "tiny-symbols.tar.gz"
            found = pack_tar_flat(archive, stage, "utw.debug")
            self.assertEqual(found, linux_symbols_entries("utw"))
            with tarfile.open(archive) as tar:
                members = {member.name: member for member in tar.getmembers()}
            self.assertEqual(members["utw.debug"].mode, 0o644)
            member = members["utw.debug"]
            self.assertEqual((member.uid, member.gid, member.uname, member.gname),
                             (0, 0, "", ""))


if __name__ == "__main__":
    unittest.main()
