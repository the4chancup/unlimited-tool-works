# Archive-source fixtures (step 3.8d)

Written by the lead with Python's `zipfile` (fixed timestamps) and 7-Zip 7z.exe, from a script
kept in the session scratch (`.tmp/3_8/make_source_fixtures.py`, not part of the repository).

| File | Contents | For |
|---|---|---|
| `co Midcup Spring.zip` | `players.txt` (`01 Keeper`), `notes.txt`, directory entries `Kits/` and `Kits/p2/`, deflate, no wrapper folder | TC-SRC-01: the same export as a folder (built by the test: git keeps no empty folder) and as a `.7z` |
| `co Midcup Spring.7z` | the same files and folders, solid LZMA2 (`7z a -t7z -ms=on -m0=lzma2`; 7-Zip reports `Solid = +`) | TC-SRC-01, and the solid-7z memory charge (`libs/pipeline.md`) |
| `co Midcup Case.zip` | `players.txt` and `Players.txt` | TC-SRC-07: two names that fold to one |
| `co Midcup Escape.zip` | `players.txt` and `../x` | TC-SRC-07: a path escaping the root |

Added at step 3.9c, by the lead, from `.tmp/4_0/make_tracer_archives.py` (same tools; the eight files
`../tracer/studio/egg Midcup Tracer/` held then, at the archive root; the `portrait.dds` it gained at
step 4.2 is not in them, and the tests over these archives assert faces and kits only;
rewritten at step 4.30 by `.tmp/4_30/make_tracer_archives_4_30.py`, the kit's icon file becoming the
empty marker `Kits/g1/icon_11`, every other entry's bytes kept; rebuilt at step 0.0.2-poc12 by
`scripts/provenance/fixtures/tracer_archives_team_folder.py` with the tracer's respelled
`fcl_hair.fmdl`, every other entry kept, the bad CRC included):

| File | Contents | For |
|---|---|---|
| `egg Midcup Tracer.zip` | the tracer export, deflate | compile from a zip (TC-ID-01 copies it under another name) |
| `egg Midcup Tracer.7z` | the tracer export, solid LZMA2 | compile from a `.7z`: its tasks share one permit, and one over the memory cap still compiles |
| `egg Midcup Tracer bad notes.zip` | the tracer export plus a stored `notes.txt` whose CRC-32 is wrong in its local header and the central directory | TC-ROOT-05: that one read fails its checksum, the listing and every other entry read fine |
