# Format census kit

A format crate's census (RUNBOOK.md "Closing a phase"): every file of that format on the
maintainer's machine run through the crate's read, its semantic round trip and `check`, tallied
by outcome, each failing class diagnosed, and the census re-run after every fix with a
regression check. These three scripts are the parts that do not depend on the format:

| Script | Does |
|---|---|
| `list_files.py "<Everything search>" <out.txt>` | the input list, from the Everything index (archives are opened by the run, not listed here) |
| `summarize.py <outcomes.tsv>` | the tally: files per outcome class, the smallest of each first |
| `regress_check.py <before.tsv> <after.tsv>` | after a fix: no file that read `ok` before reads otherwise now, and every outcome change |

The run itself is the format's own: a test binary or CLI command that reads each listed path and
writes one `outcome<TAB>size<TAB>path` row per file to an outcomes TSV, kept in the census's
scratch folder (`.tmp/<step>/…_census/`) with its probes. The first census, PES 2017's `.model`
files at 2.20g, ran this way; `scripts/library_parity/` is the export upgrader's own census, with
its classes and archives.

## The census index

A question about the real files ("does any FMDL name a directory like this", "how many
uncompressed textures have a side that is no multiple of 4") is answered from a kept index, not
a new walk of the disk. `index.py` walks the roots once and writes `.census/index.sqlite` at the
repository root; `query.py` answers named questions from it in seconds. `.census/` is
gitignored and is not scratch: a cleanup of `.tmp/` leaves it.

**A count the spec or a decision cites is cited as its query command, not as a scratch file.**
A scratch file is gone after the next cleanup; the command reproduces the count on any day.

### Building and refreshing

`python scripts/census/index.py` (or `just census-index`). The roots are the lines of
`.census/roots.txt` (`#` starts a comment); when the file is missing the script writes one
holding `C:/Data/4cc` and says so. A rerun re-reads only the files whose size or modification
time changed and drops the rows of files that are gone, so it costs about one walk.

The walk skips folders named `target`, `.git` or `node_modules`, and any folder holding a
`.git` entry (a repository or a worktree: their fixtures are not the library); the index
counts the folders each rule dropped. A root is walked whatever it holds. `--keep-repositories`
walks repositories too, and `--index FILE` / `--roots FILE` keep such an index apart from the
default one.

### What the index holds

| Table | Rows |
|---|---|
| `files` | every file under the roots: `path` (forward slashes), `root`, `size`, `mtime`, `ext` (lowercase, with the dot), `kind` (`dds`, `ftex`, `fmdl`, `model`, `mtl`, `xml`, `other`) and `folders`, the folder names between the root and the file, lowercased, written `/faces/xxx01 - name/` |
| `textures` | one per `.dds` and `.ftex`: `width`, `height`, `levels`, `header` (`masks`, `fourcc`, `dx10`, `ftex`) with `bit_count`, `fourcc`, `dxgi` or `ftex_format`, `uncompressed`, `wesys` (the DDS sat in PES's zlib wrapper); a file whose header cannot be read has its `error` instead |
| `fmdls` | one per `.fmdl`: its count of texture references, or the `error` that stopped the read |
| `fmdl_textures` | one per texture reference of an FMDL: `directory` and `name` |
| `skips`, `folders`, `meta` | the folders each skip rule dropped; every walked folder's modification time (the staleness check); when, over which roots and in how long the index was built |

`uncompressed` is 1 when the header names no block-compressed layout: a masks header; a
FourCC other than `DXT1`–`DXT5`, `ATI1`, `ATI2`, `BC4U`, `BC4S`, `BC5U`, `BC5S`; a DX10 DXGI
format outside 70–84 and 94–99 (BC1 to BC7, every variant); an FTEX format other than 2–4 and
8–11. `dds_convert::probe`'s `Probe::uncompressed` differs in two places: it calls BC6H (DXGI
94–96, FTEX 10) uncompressed, being no codec it keeps, where the index calls it compressed;
and it refuses outright the headers it cannot convert (a cube map, a volume, an unknown FourCC
or DXGI format, a paletted or bump layout), where the index still classifies them by the rule
above.

FMDL texture references are read through the file's structure (the container's section-0
block 6, each entry's file-name and directory string ids, resolved through block 12 into the
section-1 string blocks), not by scanning for printable strings: a string that is not a
texture reference (a bone, a material, a parameter name) is never taken for one, and a
reference is never missed for its spelling. An FMDL whose container does not read has a row in
`fmdls` with the error and no references.

**Not in the index:** packed files (`.zip`, `.7z`, `.rar`, `.cpk`, `.fpk`) are not opened;
each query prints how many there are of each. A junction or symbolic link is not followed
(its target's files would count twice), and counts under the skips as `links`. A `.model` or `.mtl` is listed in `files` but not read. A file rewritten in place
changes no folder's time, so the staleness check misses it until a rerun picks it up.

### Queries

Every query prints the index's age, roots, skips, unopened packed files and unreadable files;
says `STALE` with the rerun command when an indexed folder changed since the walk, or the
roots file changed or is gone; then its
`COUNT`, a `control` or `scope` count beside it that shows the index held comparable files
(a 0 is never alone), a tally of the values counted, and examples (`--examples N`, default
10). Regular expressions are Python's, matched case-insensitively anywhere in the value;
`--in FOLDER` keeps the files inside a folder of that name at any depth; `--index FILE` asks
another index.

| Query | Counts | Example |
|---|---|---|
| `fmdl-dirs REGEX [--except REGEX] [--in FOLDER]` | FMDLs naming a texture directory that matches; a directory matching `--except` too is not counted, and the FMDLs naming one are the control | `python scripts/census/query.py fmdl-dirs "/character/common/[^/]+/" --except "sourceimages/$"` |
| `fmdl-names REGEX [--except REGEX] [--in FOLDER]` | the same over texture file names | `python scripts/census/query.py fmdl-names dummy_kit --in faces` |
| `textures [--uncompressed \| --compressed] [--odd-side] [--not-pow2] [--side-over PIXELS] [--header KIND] [--kind dds\|ftex] [--name REGEX] [--except-name REGEX] [--in FOLDER]` | textures passing every predicate (`--odd-side`: a side no multiple of 4; `--not-pow2`: a side no power of two; `--side-over`: a side longer than PIXELS); a file name matching `--except-name` is not counted, and those it removed are the control | `python scripts/census/query.py textures --kind dds --uncompressed --odd-side --except-name "portrait\|player_\|_nrm"` |
| `sql "SELECT ..."` | the statement's rows; the control is the size of each table | `python scripts/census/query.py sql "SELECT directory, count(*) FROM fmdl_textures GROUP BY directory ORDER BY 2 DESC"` |

Tests: `python scripts/census/census_test.py` (in `just test`).
