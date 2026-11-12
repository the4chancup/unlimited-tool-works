"""The machine-specific values every library-parity script reads, in one place: where Red's
checkout and 7-Zip are, which trees the input search leaves out, where the runs write. Edit
these for another machine; the scripts hold no other path of the maintainer's.
"""

import os

REPO = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))

# Where every run writes (gitignored scratch): the census under `census/`, the library run
# under `library/`, our own compile's checkpoints under `checkpoints/`, one folder per tag.
WORK = os.path.join(REPO, ".tmp", "library_parity")
CENSUS_DIR = os.path.join(WORK, "census")
LIBRARY_DIR = os.path.join(WORK, "library")
CHECKPOINT_DIR = os.path.join(WORK, "checkpoints")

# The Red checkout the library run copies for each export, and the commit it must be at (`main`,
# 4.4.0): Red's output is the parity standard, so a run against another commit is no comparison.
RED = r"C:\Data\4cc\4cc aet compiler\4cc-aet-compiler-red"
RED_COMMIT = "1a48fa5"

SEVEN_ZIP = r"C:\Program Files\7-Zip\7z.exe"
# Everything's HTTP server, which the input search queries for folders and archives by name.
EVERYTHING = "http://127.0.0.1/"

# Trees the input search leaves out: the repository, its worktrees and its copies in temp
# folders (cargo-mutants sandboxes), whose fixtures the tests already cover; the recycle bin.
EXCLUDED = ["c:\\data\\4cc\\tools_mine\\4cc-studio", "\\$recycle.bin\\",
            "c:\\users\\shakes\\appdata\\local\\temp\\"]
# Archives counted, not listed: the external drive's music collection (2,203 large solid
# archives on a spinning disk, about a minute each; no export among them).
SKIPPED = ["f:\\music\\"]

# The PES version each kind of export is compiled for, by Red and by us: the cups' versions.
PREFOX_VERSION = 17
FOX_VERSION = 21

# The folder names an old (AET) export root holds (`export_upgrader`'s `source.rs`
# `holds_old_export`).
OLD_FOLDERS = ["faces", "boots", "gloves", "kit configs", "kit textures", "common",
               "portraits", "collars", "logo", "other", "players"]
