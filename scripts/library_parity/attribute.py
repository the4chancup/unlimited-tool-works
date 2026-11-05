"""Every non-equal line of every `jobs/<n>/parity.tsv` a library run wrote, attributed to a
cause class of `classes.md` beside this script (I intentional differences, `error:` the
Error codes that dropped the content, D defect candidates), or `unattributed` for diagnosis.

A line's owner is the player slot its path names (`face/real/<team><NN>`, a common
`<NN - name>` folder, `player_<team><NN>` portraits, a boots or gloves ID the upgrade's
`wearer_assumed` lines or the compiler's exclusive range give a slot) or its kit (`u0TTTp1`).
A Red entry missing from ours (`red_only`, `*_member_missing`) whose owner slot carries an
Error in the compile log is attributed to that slot's Error codes; the other lines follow the
documented-difference rules below.

Reads and writes under `config.LIBRARY_DIR`: the jobs of `LIBRARY_JOBS` (environment, default
`jobs`, as `library.py --jobs`), and writes `attribution<ATTR_TAG>.txt` (per cause: lines,
exports, the exports' paths for the error, D and unattributed causes) and
`unattributed<ATTR_TAG>.tsv` (n, then the parity line). Usage: attribute.py <rows_<tag>.tsv>...
"""
import ast
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict

import config

HERE = config.LIBRARY_DIR
JOBS = os.environ.get("LIBRARY_JOBS", "jobs")
EQUAL = {"exact", "fmdl_equal", "ftex_equal", "dds_equal", "dds_reencoded_equal", "mtl_equal", "xml_equal",
         "uniparam_equal", "fpk_equal", "cpk_equal", "relocated", "relocated_id", "renamed_model",
         "studio_record"}
FINDING = re.compile(r"^- [^:]*: (Error|Warning|Info) (\S+)(?: at (.*?))?(?: \((.*)\))?$")
WEARER = re.compile(r"wearer_assumed at (Boots|Gloves)/([kg])(\d+).*?\(id=\d+, slot=(\d+)\)$", re.M)
# Red's item folder `k2352 - family` and the shared folder name the upgrade may make of it.
ITEM_FOLDER = re.compile(r" at (Boots|Gloves)/([kg]\d+) - (.*?) \(", re.M)
TEAM = re.compile(r"export_identified \(team=[^,]*, id=(\d+)\)")
MISSING = ("red_only", "cpk_member_missing", "fpk_member_missing")
# The exports holding game-path face folders.
GAME_PATH_EXPORTS = {"FGOG Aesthetic Export from VGL6", "FGOG Aesthetic Export minipatch.zip", "co_fetus_new_aesthetics"}


def findings(text):
    """(severity, code, slot or scope) for every finding line of a log."""
    out = []
    for line in text.splitlines():
        match = FINDING.match(line)
        if not match:
            continue
        severity, code, scope, _context = match.groups()
        scope = scope or ""
        slot = re.match(r"Players/(\d\d)", scope)
        out.append((severity, code, slot.group(1) if slot else scope))
    return out


def job_dir(index):
    """Input `index`'s job folder (an absolute `LIBRARY_JOBS` is taken as it is)."""
    return os.path.join(HERE, JOBS, str(index))


class Job:
    def __init__(self, index):
        job = job_dir(index)
        self.upgrade = open(os.path.join(job, "upgrade.txt"), encoding="utf-8").read()
        self.compile = open(os.path.join(job, "compile.txt"), encoding="utf-8").read()
        self.findings = findings(self.compile)
        team = TEAM.search(self.compile)
        self.team = int(team.group(1)) if team else None
        # The upgrade's Kit Textures leftovers, spelled as Red renames them (`u0???` -> the team).
        self.leftover_kit_stems = set()
        for name in re.findall(r"leftover_dropped at Kit Textures/(?:[^/\n]*/)?([^/\n]+?)\.\w+$", self.upgrade, re.M):
            if self.team is not None:
                self.leftover_kit_stems.add(re.sub(r"u0[a-z0-9]{3}([pg])", rf"u0{self.team}\1", name.lower()))
        self.full = bool(re.search(r"^- \S+ Full(?![a-z])", self.compile, re.M))
        self.errors = defaultdict(set)
        self.warnings = defaultdict(set)
        self.retargeted = set()
        self.midcup = bool(re.search(r"^- \S+ Midcup", self.compile, re.M))
        self.codes = defaultdict(set)
        self.all_codes = {code for _severity, code, _slot in self.findings}
        # An item model retargeted anywhere (the scope is a player, a shared folder or a referee).
        self.item_retargeted = bool(re.search(r"skeleton_retargeted at [^(]*\(model=(?:boots|glove_[lr])\.fmdl", self.compile))
        for severity, code, slot in self.findings:
            self.codes[slot].add(code)
            if code == "skeleton_retargeted":
                self.retargeted.add(slot)
            if severity == "Error":
                self.errors[slot].add(code)
            elif severity == "Warning":
                self.warnings[slot].add(code)
        # Red's folder ID -> slot, from the upgrade (loose mode places a folder by its ID).
        self.item_slot = {f"{letter}{folder}": slot for _kind, letter, folder, slot in WEARER.findall(self.upgrade)}
        # Red's item ID -> the shared folder scope (`Boots/family`) its folder's name gives.
        self.item_scope = {item.lower(): f"{kind}/{name}" for kind, item, name in ITEM_FOLDER.findall(self.upgrade)}
        # Slots wearing an item folder whose stray FMDL the upgrade dropped as a leftover
        # (step 10): Red discards the whole folder, ours packs the rest.
        stray = re.findall(r"leftover_dropped at (?:Boots|Gloves)/([kg]\d+)[^/\n]*/[^/\n]+\.fmdl$", self.upgrade, re.M)
        self.stray_slots = {self.item_slot[folder] for folder in stray if folder in self.item_slot}
        # Slots whose face folder Red discarded for a broken `.mtl`/`face.xml` (`xml_check.py`).
        red_run = os.path.join(job, "red_run.txt")
        text = open(red_run, encoding="utf-8", errors="replace").read() if os.path.isfile(red_run) else ""
        text = re.sub(r"\x1b\[[0-9;]*m", "", text)
        # Slot -> the reason Red's "Bad face folders" block gives (`broken mtl files`, `face.skl is
        # not allowed`).
        self.red_discarded = {slot: reason for slot, reason in
                              re.findall(r"- Face folder: +\w{3}(\d\d)\b[^\n]*\n- \(([^)]*)\)", text)}
        # Slots Red refuses for a Common path not spelled `XXX` (one "Files loaded from the
        # Common folder must use a path with" block per reference, its folder two lines down).
        self.red_common_path = set(re.findall(
            r"Files loaded from the Common folder must use a path with\n[^\n]*\n- Folder: +\w{3}(\d\d)\b", text))

    def slot_of(self, path):
        if self.team is not None:
            match = re.search(rf"face/real/{self.team}(\d\d)\b", path) or re.search(rf"player/(?:player_)?{self.team}(\d\d)\.", path)
            if match:
                return match.group(1)
        match = re.search(r"common/\d{3}/(\d\d) - ", path)
        if match:
            return match.group(1)
        # A boots or gloves package's wearer, before the kit scope: a kit-magic texture in a
        # package (`boots/k2709/u0809p1.ftex`) is the wearer's, not the kit's.
        match = re.search(r"/(?:boots|glove)/([kg]\d+)/", path)
        if match:
            item = match.group(1)
            if item in self.item_slot:
                return self.item_slot[item]
            if self.team is not None:
                position = int(item[1:]) - ((self.team - 701) * 40 + 101) + 1
                if 1 <= position <= 23:
                    return f"{position:02}"
            return None
        # A kit's scope: `u0TTTp1` textures, `TTT_DEF_1st`/`GK1st` configs.
        match = re.search(r"/u0\d{3}([pg]\d)", path.lower())
        if match:
            return f"Kits/{match.group(1)}"
        match = re.search(r"_def_(gk)?(\d)(?:st|nd|rd|th)_realuni\.bin$", path.lower())
        if match:
            return f"Kits/{'g' if match.group(1) else 'p'}{match.group(2)}"
        return None


MODEL_EXTENSIONS = (".fmdl", ".model")


SEVEN_ZIP = config.SEVEN_ZIP
ARCHIVE_LISTINGS = {}


def archive_folders(source):
    """An archive's `(kind dir, folder name) -> [file names below it]`, lowercased kind dirs,
    from `7z l -slt` (folders listed with no files are present with an empty list)."""
    if source not in ARCHIVE_LISTINGS:
        listing = subprocess.run([SEVEN_ZIP, "l", "-slt", "-ba", source], capture_output=True, text=True,
                                 encoding="utf-8", errors="replace").stdout
        folders = defaultdict(list)
        for block in listing.split("\n\n"):
            fields = dict(line.split(" = ", 1) for line in block.splitlines() if " = " in line)
            parts = fields.get("Path", "").replace("\\", "/").split("/")
            # `<export>/<kind>/<folder>[/...]`, or the export's folders at the archive root.
            for start in (0, 1):
                if len(parts) > start + 1 and parts[start].lower() in ("faces", "boots", "gloves"):
                    key = (parts[start].lower(), parts[start + 1])
                    folders.setdefault(key, [])
                    if fields.get("Folder") != "+" and len(parts) > start + 2:
                        folders[key].append(parts[-1])
                    break
        ARCHIVE_LISTINGS[source] = folders
    return ARCHIVE_LISTINGS[source]


def old_folder_files(source, kind_dir, name_test):
    """The file names below the first `<kind_dir>/<name>` subfolder of the input (a folder or
    an archive) whose name passes `name_test`, or None when there is none."""
    if os.path.isfile(source):
        for (kind, name), files in sorted(archive_folders(source).items()):
            if kind == kind_dir.lower() and name_test(name):
                return files
        return None
    root = os.path.join(source, kind_dir)
    if not os.path.isdir(root):
        return None
    for name in sorted(os.listdir(root)):
        folder = os.path.join(root, name)
        if os.path.isdir(folder) and name_test(name):
            return [n for _r, _d, ns in os.walk(folder) for n in ns]
    return None


def holds_model(files):
    return any(name.lower().endswith(MODEL_EXTENSIONS) for name in files)


def entry_lists(detail):
    """The two `face.xml` entry lists of an `xml_differs` detail, `entries [...] vs [...]`."""
    red, ours = detail[len("entries "):].split("] vs [", 1)
    # `red_parity` appends `; dif <a> vs <b> bytes` when the face diffs differ too.
    ours = ours.split("]; dif ", 1)[0].rstrip("]") + "]" if "]; dif " in ours else ours
    return ast.literal_eval(red + "]"), ast.literal_eval("[" + ours)


def kind_of(path):
    lower = path.lower()
    name = lower.rsplit("/", 1)[-1]
    if "/boots/" in lower:
        return "boots"
    if "/glove/" in lower:
        return "gloves"
    if "/face/real/" in lower:
        return "face"
    if "/uniform/texture/" in lower:
        return "kit texture"
    if "/uniform/team/" in lower and name.endswith("_realuni.bin"):
        return "kit config"
    if name in ("unicolor.bin", "teamcolor.bin", "uniformparameter.bin"):
        return "bins"
    if "/render/symbol/player/" in lower:
        return "portrait"
    if "/render/symbol/flag/" in lower:
        return "logo"
    if "/common/" in lower:
        return "common"
    return "other"


def cause_of(job, side, red, ours, class_, detail, version):
    path = red or ours
    lower = path.lower()
    kind = kind_of(path)
    slot = job.slot_of(path)
    if class_ == "dropped_fpk_xml":
        return "I1 fpk.xml dropped"
    # A game-path face folder's face.xml under Red is one Red generated at the player's path,
    # the member's tree nested below it.
    if class_ == "xml_differs" and kind == "face" and os.path.basename(job.source) in GAME_PATH_EXPORTS:
        return "I30 game-path face folder: Red's generated face.xml, ours the member's"
    # A Full slot's kit texture no config names, or a copy, is a leftover the upgrade drops.
    # Red packs it (no row loads it); ours packs nothing or the placeholder there.
    if class_ == "ours_only" and kind in ("boots", "gloves", "common") and slot in job.stray_slots:
        return "I33 an item folder with a stray FMDL: Red discards it, ours drops the file and packs the rest (step 10)"
    if class_ in ("red_only", "ftex_differs") and kind == "kit texture" and job.team is not None:
        stem = os.path.splitext(os.path.basename(lower))[0]
        if stem in job.leftover_kit_stems:
            return "I31 a kit texture the upgrade drops as a leftover (step 10): Red packs it"
    if job.midcup and class_ == "ours_only" and slot and slot.startswith("Kits/") and "kit_placeholder" in job.codes[slot]:
        return "D2 Midcup colors-only kit compiled as a placeholder"
    # A Midcup kit without a config keeps the installed one. This run installs nothing, so
    # each gets the template (`kit_config_generated`), where Red writes none.
    if job.midcup and class_ == "ours_only" and slot and slot.startswith("Kits/") and kind == "kit config" and "kit_config_generated" in job.codes[slot]:
        return "I29 Midcup kit config from the template: nothing installed in this measurement"
    if job.midcup and class_ == "ours_only" and kind == "bins" and any("kit_config_generated" in codes for codes in job.codes.values()):
        return "I29 Midcup kit config from the template: nothing installed in this measurement"
    referee = os.path.basename(job.source).lower().startswith("refs")
    if referee and (class_ == "ours_only" and (kind == "bins" or re.search(r"ref_marker\.|collar_077\.", lower))
                    or class_ == "red_only" and re.search(r"/common/999/[^/]+/sourceimages/#windx11/portrait\.ftex$", lower)):
        return ("I21 a refs export's template output: the bins and the marker (blue_port.md), "
                "a referee's portrait not packed (no referee shows one)")
    if class_ in ("cpk_differs", "fpk_differs", "relocated_id_differs") and "a member differs" in detail:
        return "container (its member lines)"
    # An Error in the owner slot drops or replaces its content; the tool then pairs Red's
    # entry with the nearest of ours, so a differing line follows from the drop too.
    item = re.search(r"/(?:boots|glove)/([kg]\d+)/", lower)
    shared = job.item_scope.get(item.group(1)) if item else None
    if shared and job.errors.get(shared):
        return "error: " + ",".join(sorted(job.errors[shared]))
    kit_errors = sorted({code for scope, codes in job.errors.items() if scope.startswith("Kits/") for code in codes})
    if class_ == "bin_records_differ" and kit_errors:
        # A failed kit keeps the entry it had in the record
        # (`spec/team_compiler/pipeline.md` "Bins accumulation").
        return "error: " + ",".join(kit_errors)
    if slot is not None and job.errors.get(slot):
        return "error: " + ",".join(sorted(job.errors[slot]))
    # A face naming the team's Common as `common/o/` was refused by Red ("broken mtl files",
    # or its Common-path block for a face.xml); ours migrates it.
    if slot is not None and (job.red_discarded.get(slot) == "broken mtl files" or slot in job.red_common_path):
        return "I34 a face whose Common path names the team (common/o/): Red discards it, ours migrates it"
    if slot is not None and "is not allowed" in job.red_discarded.get(slot, ""):
        return "I25 face folder Red discards for the other engine's file (face.skl on PES 17), ours ignores the file"
    if class_ == "red_only" and side == "refs" and kind == "face" and version == "21":
        return "I18 referee without a face model gets no face"
    if class_ == "red_only" and kind == "face" and slot is not None and not slot.startswith("Kits/"):
        if old_folder_files(job.source, "Faces", lambda name: name[3:5] == slot) == []:
            return "D1 empty face folder migrated as ingame_face"
    if side == "refs" and class_ == "red_only" and re.search(r"/(?:boots/k99|glove/g99)\d\d/#win/(boots|glove)\.fpkd?$", lower):
        return "I20 referee modelless item folder: Red packs an empty k99XX/g99XX, ours none (none shows either way)"
    if class_ == "red_only" and kind in ("boots", "gloves"):
        item = re.search(r"/(?:boots|glove)/([kg]\d+)/", lower)
        if item:
            files = old_folder_files(job.source, "Boots" if kind == "boots" else "Gloves",
                                     lambda name: name.lower().startswith(item.group(1)))
            if files is not None and not holds_model(files):
                return "I32 modelless item folder: Red packs an empty package, ours marks the wearers fpc_on (maintainer, 2026-10-19)"
    if class_ == "ours_only" and kind == "common" and os.path.basename(lower) in job.red_names:
        return "I2 relocated copy (one Red texture, a copy per wearer)"
    if class_ in ("ftex_differs", "relocated_differs") and detail.endswith("pixels equal"):
        return "I7b FTEX header canonical (type 0x9, depth 1), pixels equal"
    if class_ in ("ftex_differs", "relocated_differs") and re.search(r"format: Argb8.*ours FtexInfo \{ version: 2\.04, format: Bc7", detail):
        return "I8 uncompressed re-encoded"
    if class_ == "fmdl_texture_names_differ" or (class_ == "ours_only" and re.search(r"(^|[_ ])kit(n|f|\d)([_.]|$)", os.path.basename(lower))) \
            or (class_ == "cpk_member_missing" and re.search(r"u0\w{3}[pg]\d", os.path.basename(lower))):
        return "I4 kitN spelling"
    if job.full and (class_ in ("uniparam_differs", "bin_records_differ")
                     or (class_ == "ours_only" and kind in ("kit texture", "kit config", "bins"))):
        return "I5 Full placeholders and rebuilt bins"
    if class_ == "kitconfig_differs":
        return "I6 kit config names"
    if class_ == "ftex_differs" and re.search(r"_(back|name|leg|chest)\.ftex$", lower):
        return "I7 number atlas re-arranged"
    if "dds_reencoded" in class_ or "dds_reencoded" in detail:
        return "I8 uncompressed re-encoded"
    if kind == "logo" and class_ == "bytes_differ":
        return "I9 logos re-encoded"
    if kind == "portrait" and class_ in ("dds_differs", "relocated_differs"):
        return "I9 portraits re-encoded"
    if class_ == "cpk_member_missing" and lower.endswith((".model", ".mtl")) and "xml_model_unlisted" in job.warnings.get(slot, ()):
        return "I10 unlisted model not packed"
    if side == "refs" and version == "17" and (class_ in ("cpk_member_ours_only", "red_only", "xml_differs", "cpk_member_missing")
                                               or class_ == "ours_only" and "/common/999/refkit/" in lower):
        return "I11 pre-Fox referee layout"
    if lower.endswith("face_diff.bin") and class_ == "bytes_differ" or (class_ == "xml_differs" and detail.startswith("dif ")):
        return "I12 template face_diff"
    if class_ == "red_only" and kind == "kit texture" and (re.search(r"g[2-9](_mask)?\.", lower) or "_mask." in lower):
        return "I13 mask or extra GK kit"
    if class_ == "red_only" and kind == "kit texture" and slot and "kit_texture_not_used" in job.codes.get(slot, ()):
        return "I13b kit texture under a name the game never loads (_front, _font, _leg .dds): kit_texture_not_used, not packed"
    if class_ == "bytes_differ" and lower.endswith(".model") and red and ours and os.path.dirname(red.replace(".cpk/", "/")).lower() != os.path.dirname(ours).lower():
        return "I10 unlisted model not packed"
    common_model = "/uniform/common/" in lower and slot is None and any(scope.startswith("Common") for scope in job.retargeted)
    if class_ in ("fmdl_differs", "bytes_differ") and lower.endswith((".fmdl", ".skl", ".model")) and (slot in job.retargeted or common_model):
        return "I14 re-bound through the IR (skeleton_retargeted)"
    if class_ == "fmdl_differs" and ("model_hand_split" in job.all_codes or "fmdl_merged" in job.all_codes) and (kind == "gloves" or "fcl_hair" in lower):
        return "I23 hand auto-split: a face model's hands moved into the gloves (model_hand_split, fmdl_merged)"
    if class_ in ("fmdl_differs", "bytes_differ") and kind in ("boots", "gloves") and lower.endswith((".fmdl", ".skl")) and job.item_retargeted:
        return "I14 re-bound through the IR (skeleton_retargeted)"
    if class_ == "fmdl_differs" and re.match(r"(?:materials (\[.*?\]) vs \1, )?textures \[", detail):
        textures = re.search(r"textures (\[.*?\]) vs (\[.*?\])", detail)
        if textures and textures.group(1).lower() == textures.group(2).lower() and ", meshes" not in detail:
            return "I24 FMDL texture names keep their letter case (Red lowercases a kit-magic name)"
    if class_ == "fmdl_differs":
        return "I14? fmdl differs (re-bound or paired)"
    if class_ == "xml_differs" and detail.startswith("entries "):
        red_entries, ours_entries = entry_lists(detail)
        if sorted((t, m) for t, _p, m in red_entries) == sorted((t, m) for t, _p, m in ours_entries):
            return "I19 face.xml model names (same types and materials, each names its own file)"
    if class_ in ("ftex_differs", "relocated_differs") and re.search(r"(_nrm|normal)[^/]*\.ftex$", lower) and re.search(r"ours FtexInfo \{ version: [\d.]+, format: Bc3", detail):
        return "I26 a normal map re-encoded as DXT5nm BC3 (dds_convert.md 'DXT5nm handling')"
    tail = re.search(r"same headers, pixels differ in [\d.]+% of (\d+) blocks \((\d+) blocks, first #(\d+)", detail)
    if class_ in ("ftex_differs", "relocated_differs") and tail and int(tail.group(3)) >= int(tail.group(1)) - 64:
        # Every differing block in the payload's last 64: the smallest mips only.
        return "I15 last two BC blocks"
    if class_ == "ours_only" and kind == "gloves" and "model_hand_split" in job.all_codes:
        return "I23 hand auto-split: a face model's hands moved into the gloves (model_hand_split, fmdl_merged)"
    if class_ == "fpk_member_ours_only" and "fcl_hair" in lower and "fmdl_fcl_hair_fallback" in job.codes.get(slot, ()):
        return "D9 a stray FMDL copy (boots - Copy.fmdl) migrated into the player folder, merged as hair"
    if class_ == "fpk_differs" and "fcl_hair" in detail and "fmdl_fcl_hair_fallback" in job.codes.get(slot, ()):
        return "D9 a stray FMDL copy (boots - Copy.fmdl) migrated into the player folder, merged as hair"
    if class_ == "xml_differs" and detail.startswith("entries "):
        red_entries, ours_entries = entry_lists(detail)
        kept = [(t, m) for t, p, m in ours_entries if "hair_high" not in p]
        if sorted(kept) == sorted((t, m) for t, _p, m in red_entries):
            return "I19b a generated face.xml lists hair_high as parts (Red leaves it to the game's by-name load)"
    if class_ == "red_only" and kind == "common" and "file_not_used" in job.all_codes and re.search(
            rf"file_not_used \(file=Common/[^)]*{re.escape(os.path.splitext(os.path.basename(lower))[0])}\.", job.compile, re.I):
        return "I28 a Common file nothing references is not packed (Common is a library; file_not_used)"
    # The same file, paired by `red_parity` with the nearest of ours (it pairs every Red entry
    # it can): Red's Common path, its kit folder `u0XXXpN` read as the upgrade's `kitN`, must be
    # the very file the compile reports unused (GBPEN VGL26's per-kit 8x8 dummies, close6).
    common = re.search(r"/common/\w{3}/(.+?)(?:/sourceimages)?(?:/#windx11)?/([^/]+)\.ftex$", lower)
    if class_ == "relocated_differs" and kind == "common" and common:
        folder = re.sub(r"^u0\w{3}p(\d)$", r"kit\1", common.group(1))
        if re.search(rf"file_not_used \(file=Common/{re.escape(folder)}/{re.escape(common.group(2))}\.", job.compile, re.I):
            return "I28 a Common file nothing references is not packed (Common is a library; file_not_used)"
    if side == "refs" and version == "17" and class_ == "relocated_differs" and "/common/999/" in lower:
        return "I11 pre-Fox referee layout"
    if class_ == "ours_only" and lower.endswith(".dds") and re.search(
            rf"{re.escape(os.path.splitext(os.path.basename(lower))[0])}\.png", " ".join(job.red_names)):
        return "I27 a PNG source converted to the .dds its material names (Red packs the PNG, which the game cannot read)"
    # A pre-Fox Common model already named `oral_x`: Red prefixes it again, ours keeps the name.
    # Each side's line needs the other's file, so a model ours dropped for another cause stays
    # unattributed (U AAE 26, c4).
    base = os.path.basename(lower)
    if kind == "common" and base.endswith(".model") and (
            (class_ == "red_only" and base.startswith("oral_oral_") and base[len("oral_"):] in job.ours_names)
            or (class_ == "ours_only" and base.startswith("oral_") and "oral_" + base in job.red_names)):
        return "I35 a Common model named oral_x: Red packs it as oral_oral_x, ours as oral_x"
    # The fallbacks: what no class above claims.
    if class_ == "cpk_member_missing" and lower.endswith((".txt", ".png")):
        return "I22 a file the game never reads (.txt, .png): Red packs it, ours reports file_not_used"
    if class_ == "cpk_member_missing" and lower.endswith(".mtl"):
        return "I10b an .mtl no packed model uses is not packed (Red's paired with a same-named one elsewhere)"
    if class_ == "mtl_differs" and red and ours and os.path.dirname(red.replace(".cpk/", "/")).lower() != os.path.dirname(ours).lower():
        return "I10b an .mtl no packed model uses is not packed (Red's paired with a same-named one elsewhere)"
    if class_ in MISSING and slot is not None and job.warnings.get(slot):
        return "warning-slot " + class_ + ": " + ",".join(sorted(job.warnings[slot]))
    return f"unattributed {class_} {kind}"


def main():
    rows = {}
    for file in sys.argv[1:]:
        for line in open(file, encoding="utf-8"):
            fields = line.rstrip("\n").split("\t")
            rows[int(fields[0])] = fields
    lines, exports = Counter(), defaultdict(set)
    unattributed = []
    for index in sorted(rows):
        report = os.path.join(job_dir(index), "parity.tsv")
        if not os.path.isfile(report):
            continue
        job = Job(index)
        job.source = rows[index][1]
        version = rows[index][2]
        job.red_names = {os.path.basename(line.split("\t")[1].lower()) for line in open(report, encoding="utf-8") if line.split("\t")[1]}
        job.ours_names = {os.path.basename(line.split("\t")[2].lower()) for line in open(report, encoding="utf-8") if line.split("\t")[2]}
        for line in open(report, encoding="utf-8"):
            side, red, ours, class_, detail = line.rstrip("\n").split("\t")
            if class_ in EQUAL:
                cause = "equal"
            else:
                cause = cause_of(job, side, red, ours, class_, detail, version)
            lines[cause] += 1
            exports[cause].add(index)
            if cause.startswith("unattributed") or cause.startswith("warning-slot") or cause.startswith("I14?"):
                unattributed.append(f"{index}\t{cause}\t{line.rstrip()}")
    suffix = os.environ.get("ATTR_TAG", "")
    with open(os.path.join(HERE, "attribution.txt.new"), "w", encoding="utf-8", newline="\n") as handle:
        for cause in sorted(lines, key=lambda c: -lines[c]):
            handle.write(f"{cause}: {lines[cause]} lines, {len(exports[cause])} exports\n")
            if cause.startswith(("error", "unattributed", "warning-slot", "I14?", "D")):
                for index in sorted(exports[cause]):
                    handle.write(f"    [{index}] {rows[index][1]}\n")
    os.replace(os.path.join(HERE, "attribution.txt.new"), os.path.join(HERE, f"attribution{suffix}.txt"))
    with open(os.path.join(HERE, "unattributed.tsv.new"), "w", encoding="utf-8", newline="\n") as handle:
        handle.write("".join(line + "\n" for line in unattributed))
    os.replace(os.path.join(HERE, "unattributed.tsv.new"), os.path.join(HERE, f"unattributed{suffix}.tsv"))
    for cause in sorted(lines, key=lambda c: -lines[c]):
        print(f"{lines[cause]:6} {len(exports[cause]):4}  {cause}")


if __name__ == "__main__":
    main()
