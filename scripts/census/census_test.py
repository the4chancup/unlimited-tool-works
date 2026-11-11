"""Tests of `index.py` and `query.py`: `python scripts/census/census_test.py`."""

import contextlib
import io
import os
import re
import shutil
import sqlite3
import struct
import sys
import tempfile
import unittest
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import index  # noqa: E402
import query  # noqa: E402

FIXTURES = Path(__file__).resolve().parents[2] / "crates" / "libs" / "fmdl" / "tests" / "fixtures"
COMMON = "/Assets/pes16/model/character/common/"


def dds(width, height, fourcc=b"", bit_count=0, dxgi=None):
    """A DDS header and a few bytes of pixels: a FourCC one when `fourcc` is given (`DX10`
    with `dxgi`), else a masks one of `bit_count` bits."""
    header = bytearray(128)
    header[:4] = b"DDS "
    struct.pack_into("<I", header, 4, 124)
    struct.pack_into("<2I", header, 12, height, width)
    struct.pack_into("<I", header, 28, 1)
    struct.pack_into("<I4sI", header, 80, 0x4 if fourcc else 0x40, fourcc.ljust(4, b"\0"), bit_count)
    if dxgi is not None:
        header += struct.pack("<5I", dxgi, 3, 0, 1, 0)
    return bytes(header) + bytes(64)


def ftex(pixel_format, width, height):
    return (b"FTEX" + struct.pack("<f4HB", 2.03, pixel_format, width, height, 1, 1)).ljust(64, b"\0")


def wesys(data):
    packed = zlib.compress(data)
    return b"\x00\x10\x01WESYS" + struct.pack("<2I", len(packed), len(data)) + packed


def fmdl(references):
    """An FMDL container holding only texture references: block 6 (file name and directory
    string ids), block 12 (the string records) and section-1 block 3 (the strings)."""
    records, table = [], bytearray()

    def string(text):
        records.append(struct.pack("<2HI", 3, len(text), len(table)))
        table.extend(text.encode() + b"\0")
        return len(records) - 1

    textures = b"".join(struct.pack("<2H", string(name), string(directory)) for directory, name in references)
    section0 = textures + b"".join(records)
    descriptors = (struct.pack("<2HI", 6, len(references), 0)
                   + struct.pack("<2HI", 12, len(records), len(textures))
                   + struct.pack("<3I", 3, 0, len(table)))
    descriptors += bytes(-len(descriptors) % 16)
    offset0 = 64 + len(descriptors)
    header = struct.pack("<4sI3Q6IQ", b"FMDL", 0x4001EB85, 64, 0, 0, 2, 1,
                         offset0, len(section0), offset0 + len(section0), len(table), 0)
    return header + descriptors + section0 + bytes(table)


TREE = {
    "Faces/XXX01/oral.fmdl": (FIXTURES / "addon_oral.fmdl").read_bytes(),
    "Faces/XXX01/hair.fmdl": fmdl([(COMMON + "000/hair/", "hair_bsm.dds"), ("./", "hair_nrm.dds")]),
    "Collars/Collar01/collar.fmdl": fmdl([(COMMON + "sourceimages/", "dummy_kit.dds")]),
    "Common/boots.fmdl": fmdl([(COMMON + "000/Boots/sourceimages/", "boots.dds")]),
    "Common/broken.fmdl": b"FMDL" + bytes(10),
    "Kits/u0abcp0.dds": dds(8, 8, b"DXT1"),
    "Kits/bc7.dds": dds(16, 16, b"DX10", dxgi=98),
    "Kits/odd_masks.dds": dds(6, 4, bit_count=32),
    "Kits/kit_nrm.dds": dds(6, 6, bit_count=32),
    "Kits/wrapped.dds": wesys(dds(12, 12, bit_count=32)),
    "Kits/flat.ftex": ftex(0, 12, 20),
    "Kits/broken.dds": b"not a texture",
    "Portraits/player_123.dds": dds(10, 10, b"DX10", dxgi=28),
    "archive.zip": b"",
    "target/skipped.dds": dds(4, 4, b"DXT1"),
    "repo/.git/HEAD": b"ref: refs/heads/main\n",
    "repo/inside.fmdl": fmdl([(COMMON + "000/x/", "x.dds")]),
}


class Census(unittest.TestCase):
    def setUp(self):
        self.folder = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.folder)
        self.root = self.folder / "library"
        for name, data in TREE.items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        self.roots = self.folder / "roots.txt"
        self.roots.write_text(f"# test roots\n{self.root.as_posix()}\n", encoding="utf-8")
        self.index = self.folder / "index.sqlite"

    def build(self, keep_repositories=False):
        return index.build(self.roots, self.index, keep_repositories)

    def ask(self, *argv):
        """The query's output, and the number on its COUNT, control and scope lines."""
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            query.main([*argv, "--index", str(self.index)])
        text = out.getvalue()
        numbers = {match[1]: int(match[2].replace(",", ""))
                   for match in re.finditer(r"^(COUNT|control|scope): ([\d,]+)", text, re.M)}
        return text, numbers

    def test_fmdl_directories_below_common_count_beside_the_ones_the_exception_removed(self):
        self.build()
        text, numbers = self.ask("fmdl-dirs", COMMON + "[^/]+/", "--except", "sourceimages/$")
        self.assertEqual(numbers, {"COUNT": 1, "control": 3, "scope": 4})
        self.assertIn("4 FMDLs read, 0 of them naming no texture; 1 unreadable", text)
        self.assertIn(f"1  {COMMON}000/hair/", text)

    def test_a_folder_limit_keeps_only_fmdls_inside_a_folder_of_that_name(self):
        self.build()
        _, faces = self.ask("fmdl-dirs", "common/[^/]+/", "--except", "common/sourceimages/", "--in", "faces")
        self.assertEqual(faces, {"COUNT": 1, "control": 1, "scope": 2})
        _, collars = self.ask("fmdl-names", "dummy_kit", "--in", "Collars")
        self.assertEqual(collars, {"COUNT": 1, "scope": 1})
        text, common = self.ask("fmdl-dirs", "^[a-z0-9_].*/$", "--in", "common")
        self.assertEqual(common, {"COUNT": 0, "scope": 1})
        self.assertIn("1 unreadable", text)

    def test_textures_by_predicate_and_name(self):
        self.build()
        cases = [
            (["--kind", "dds", "--uncompressed"], {"COUNT": 4, "scope": 6}),
            (["--kind", "dds", "--compressed"], {"COUNT": 2, "scope": 6}),
            (["--uncompressed", "--odd-side", "--except-name", "portrait|player_|_nrm"],
             {"COUNT": 1, "control": 2, "scope": 7}),
            (["--odd-side", "--name", "player_"], {"COUNT": 1, "scope": 1}),
            (["--not-pow2"], {"COUNT": 5, "scope": 7}),
            (["--side-over", "15"], {"COUNT": 2, "scope": 7}),
            (["--header", "dx10"], {"COUNT": 2, "scope": 7}),
            (["--kind", "ftex"], {"COUNT": 1, "scope": 1}),
        ]
        for argv, expected in cases:
            with self.subTest(argv=argv):
                self.assertEqual(self.ask("textures", *argv)[1], expected)
        text, _ = self.ask("textures", "--kind", "dds", "--uncompressed")
        self.assertIn("1 unreadable", text)
        self.assertIn("12x12 masks 32-bit", text)
        self.assertIn("10x10 DX10 DXGI 28", text)

    def test_header_rows(self):
        self.build()
        _, numbers = self.ask("sql", "SELECT t.wesys, t.header, t.fourcc FROM textures t JOIN files f "
                                     "ON f.id = t.file_id WHERE f.path LIKE '%wrapped.dds' AND t.wesys = 1 "
                                     "AND t.header = 'masks'")
        self.assertEqual(numbers["COUNT"], 1)
        text, numbers = self.ask("sql", "SELECT error FROM textures WHERE error IS NOT NULL")
        self.assertEqual(numbers["COUNT"], 1)
        self.assertIn("ValueError: no DDS header", text)

    def test_repositories_and_named_folders_are_skipped_and_counted(self):
        self.build()
        text, numbers = self.ask("sql", "SELECT path FROM files WHERE path LIKE '%inside.fmdl' OR path LIKE '%skipped.dds'")
        self.assertEqual(numbers["COUNT"], 0)
        self.assertIn("target 1, .git 0, node_modules 0, unreadable 0, links 0; repositories 1 skipped", text)
        self.assertIn("packed files not opened: 1 .zip, 0 .7z, 0 .rar, 0 .cpk, 0 .fpk", text)
        self.build(keep_repositories=True)
        text, numbers = self.ask("sql", "SELECT path FROM files WHERE path LIKE '%inside.fmdl'")
        self.assertEqual(numbers["COUNT"], 1)
        self.assertIn("target 1, .git 1, node_modules 0, unreadable 0, links 0; repositories walked", text)

    def test_every_packed_kind_is_counted_as_not_opened(self):
        for ext in (".7z", ".rar", ".cpk", ".fpk"):
            (self.root / f"packed{ext}").write_bytes(b"")
        self.build()
        self.assertIn("packed files not opened: 1 .zip, 1 .7z, 1 .rar, 1 .cpk, 1 .fpk", self.ask("textures")[0])

    @unittest.skipUnless(os.name == "nt", "NTFS junctions exist on Windows only")
    def test_a_junction_is_not_followed_and_counts_as_a_link(self):
        import _winapi
        root = self.folder / "junctioned"
        (root / "real").mkdir(parents=True)
        (root / "real" / "kit.dds").write_bytes(dds(4, 4, b"DXT1"))
        _winapi.CreateJunction(str(root / "real"), str(root / "link"))
        self.roots.write_text(f"{root.as_posix()}\n", encoding="utf-8")
        self.assertEqual(self.build()["files"], 1)
        text, numbers = self.ask("sql", "SELECT rule FROM skips WHERE rule = 'links' AND folders = 1")
        self.assertEqual(numbers["COUNT"], 1)
        self.assertIn("unreadable 0, links 1;", text)

    def test_an_index_written_before_the_links_rule_still_describes_itself(self):
        self.build()
        db = sqlite3.connect(self.index)
        db.execute("DELETE FROM skips WHERE rule = 'links'")
        db.commit()
        db.close()
        self.assertIn("unreadable 0, links 0;", self.ask("textures")[0])

    def test_a_rerun_rereads_changed_files_and_drops_gone_ones(self):
        first = self.build()
        self.assertEqual((first["files"], first["reread"], first["dropped"]), (14, 14, 0))
        self.assertEqual(self.build()["reread"], 0)
        changed = self.root / "Kits/odd_masks.dds"
        changed.write_bytes(dds(8, 4, bit_count=32))
        os.utime(changed, (1_000_000_000, 1_000_000_000))
        (self.root / "Kits/kit_nrm.dds").unlink()
        rerun = self.build()
        self.assertEqual((rerun["files"], rerun["reread"], rerun["dropped"]), (13, 1, 1))
        _, numbers = self.ask("textures", "--odd-side")
        self.assertEqual(numbers, {"COUNT": 1, "scope": 6})

    def test_a_root_inside_another_root_indexes_its_files_once(self):
        self.roots.write_text(f"{self.root.as_posix()}\n{self.root.as_posix()}/Kits\n", encoding="utf-8")
        self.assertEqual(self.build()["files"], 14)

    def test_an_index_older_than_its_roots_says_so_and_offers_the_rerun(self):
        self.build()
        self.assertIn("current: no indexed folder changed", self.ask("textures")[0])
        (self.root / "Kits" / "new.dds").write_bytes(dds(4, 4, b"DXT1"))
        text, _ = self.ask("textures")
        self.assertIn("STALE: 1 folders changed since the walk", text)
        self.assertIn(f"rerun: python scripts/census/index.py --roots {self.roots.as_posix()} "
                      f"--index {self.index.as_posix()}", text)
        self.roots.write_text(f"{self.root.as_posix()}/Kits\n", encoding="utf-8")
        self.assertIn("STALE: the roots file changed", self.ask("textures")[0])

    def test_the_rerun_command_spells_its_paths_as_git_bash_takes_them(self):
        meta = {"roots_file": str(self.roots), "keep_repositories": "0"}
        command = query.rerun_command(meta, self.folder / "my index.sqlite")
        self.assertNotIn("\\", command)
        self.assertEqual(command, f"python scripts/census/index.py --roots {self.roots.as_posix()} "
                                  f'--index "{self.folder.as_posix()}/my index.sqlite"')

    def test_a_deleted_roots_file_is_named_and_the_rerun_leaves_it_out(self):
        self.build()
        self.roots.unlink()
        text, _ = self.ask("textures")
        self.assertIn(f"STALE: the roots file {self.roots.as_posix()} is gone; "
                      f"rerun: python scripts/census/index.py --index {self.index.as_posix()}\n", text)
        self.assertNotIn("--roots", text)

    def test_a_missing_roots_file_is_written_with_the_library_root(self):
        roots = self.folder / "census" / "roots.txt"
        with contextlib.redirect_stdout(io.StringIO()) as out:
            self.assertEqual(index.read_roots(roots), ["C:/Data/4cc"])
        self.assertIn("wrote", out.getvalue())
        self.assertEqual(index.read_roots(roots), ["C:/Data/4cc"])


class FmdlReader(unittest.TestCase):
    def test_a_konami_fmdl_lists_its_four_texture_references(self):
        references = index.read_fmdl_textures(FIXTURES / "konami_mouth.fmdl")
        self.assertEqual(references, [(COMMON + "sourceimages/", f"oral_{map_}.tga")
                                      for map_ in ("bsm", "nrm", "srm", "trm")])

    def test_a_file_without_texture_references_lists_none(self):
        self.assertEqual(index.read_fmdl_textures(FIXTURES / "konami_au_Low_parts.fmdl"), [])


if __name__ == "__main__":
    unittest.main()
