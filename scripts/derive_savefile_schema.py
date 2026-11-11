"""Derive pes_savefile's schema tables from the reference editor's read walks.

Usage: python scripts/derive_savefile_schema.py <reference source dir> <crate>/src/schema

The reference source (the C++ save editor's `pes15.cpp` .. `pes20.cpp`) is not part of this
repository. Each `fill_*` / `read_*` walk is interpreted over a symbolic byte array: a value is a
list of bit slots, each None (a constant zero bit) or (byte, bit) naming the record byte and bit
it came from. `data[current_byte]`, shifts, masks, `+`/`|` and `read_data*(start, bits, ...)` are
evaluated on those slots, every assignment to a model field records which record bits feed which
field bits, and each field must end up one contiguous LSB-first run. The result is written as
`fields.rs` and `pes15.rs` .. `pes20.rs`; the plan section "Schema-driven save codec" in
`docs/spec/pes_savefile/codec.md` holds the method and the checks made against real saves.
"""
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path


CASTS = re.compile(r"\((?:byte|bool|char|int|unsigned char|unsigned long|unsigned int|uint16_t|LPCSTR|const char\s*\*)\)")
DATA = re.compile(r"(?:pDescriptor\w*->)?data\[current_byte(?:\s*\+\s*(\d+))?\]")
READ = re.compile(r"read_data(?:Old|15|_raw)?\(([^,]+),([^,]+),\s*current_byte\s*,[^)]*\)")
OBJ_PREFIX = re.compile(r"^(?:players|player|team|gteams\[t_ind\])\.")


class Slots:
    def __init__(self, slots):
        self.slots = list(slots)

    def shr(self, n):
        return Slots(self.slots[n:] + [None] * n)

    def shl(self, n):
        return Slots(([None] * n + self.slots)[:32])

    def mask(self, m):
        return Slots([s if (m >> i) & 1 else None for i, s in enumerate(self.slots)])

    def merge(self, other, where):
        out = []
        for a, b in zip(self.slots, other.slots):
            if a is not None and b is not None:
                raise ValueError(f"overlapping bits in {where}: {a} vs {b}")
            out.append(a if a is not None else b)
        return Slots(out)


class Walk:
    def __init__(self, source, name):
        self.source = source
        self.name = name
        self.cb = 0
        self.fields = {}      # path -> Slots
        self.strings = {}     # path -> (start, len)
        self.locals = {}      # name -> Slots
        self.pending_string = None
        self.order = []

    # ---- expressions -------------------------------------------------------------------------
    def data_byte(self, offset=0):
        b = self.cb + offset
        return Slots([(b, i) for i in range(8)] + [None] * 24)

    def read_data(self, start, bits):
        slots = []
        for i in range(bits):
            pos = start + i
            slots.append((self.cb + pos // 8, pos % 8))
        self.cb += (start + bits) // 8
        return Slots(slots + [None] * (32 - len(slots)))

    def eval_int(self, text):
        text = text.strip()
        return int(eval(text.replace("/", "//"), {"__builtins__": {}}, {}))  # literals and arithmetic only

    def tokenize(self, expr):
        expr = CASTS.sub("", expr)
        tokens = []
        i = 0
        while i < len(expr):
            c = expr[i]
            if c.isspace():
                i += 1
            elif expr.startswith(">>", i) or expr.startswith("<<", i):
                tokens.append(expr[i : i + 2]); i += 2
            elif c in "()&+|":
                tokens.append(c); i += 1
            else:
                m = DATA.match(expr, i)
                if m:
                    tokens.append(("data", int(m.group(1) or 0))); i = m.end(); continue
                m = READ.match(expr, i)
                if m:
                    tokens.append(("read", self.eval_int(m.group(1)), self.eval_int(m.group(2)))); i = m.end(); continue
                m = re.match(r"0x[0-9A-Fa-f]+|\d+", expr[i:])
                if m:
                    tokens.append(("int", int(m.group(0), 0))); i += m.end(); continue
                m = re.match(r"[A-Za-z_]\w*", expr[i:])
                if m:
                    tokens.append(("ident", m.group(0))); i += m.end(); continue
                raise ValueError(f"cannot tokenize {expr!r} at {i} in {self.name}")
        return tokens

    def parse(self, tokens):
        # precedence: | lowest, then &, then + , then shifts, then primary
        self.pos = 0
        self.tokens = tokens
        value = self.parse_or()
        if self.pos != len(tokens):
            raise ValueError(f"trailing tokens {tokens[self.pos:]} in {self.name}")
        return value

    def peek(self):
        return self.tokens[self.pos] if self.pos < len(self.tokens) else None

    def take(self):
        t = self.tokens[self.pos]
        self.pos += 1
        return t

    def parse_or(self):
        left = self.parse_and()
        while self.peek() == "|":
            self.take()
            left = self.combine(left, self.parse_and())
        return left

    def parse_and(self):
        left = self.parse_add()
        while self.peek() == "&":
            self.take()
            right = self.parse_add()
            left = self.apply_mask(left, right)
        return left

    def parse_add(self):
        left = self.parse_shift()
        while self.peek() == "+":
            self.take()
            left = self.combine(left, self.parse_shift())
        return left

    def parse_shift(self):
        left = self.parse_primary()
        while self.peek() in (">>", "<<"):
            op = self.take()
            right = self.parse_primary()
            if not isinstance(right, int):
                raise ValueError(f"shift by non-constant in {self.name}")
            left = left.shr(right) if op == ">>" else left.shl(right)
        return left

    def parse_primary(self):
        t = self.take()
        if t == "(":
            v = self.parse_or()
            assert self.take() == ")"
            return v
        kind = t[0]
        if kind == "data":
            return self.data_byte(t[1])
        if kind == "read":
            return self.read_data(t[1], t[2])
        if kind == "int":
            return t[1]
        if kind == "ident":
            if t[1] in self.locals:
                return self.locals[t[1]]
            raise ValueError(f"unknown identifier {t[1]} in {self.name}")
        raise ValueError(f"unexpected token {t} in {self.name}")

    def apply_mask(self, a, b):
        if isinstance(a, int) and isinstance(b, int):
            return a & b
        if isinstance(b, int):
            return a.mask(b)
        if isinstance(a, int):
            return b.mask(a)
        raise ValueError(f"mask of two symbolic values in {self.name}")

    def combine(self, a, b):
        if isinstance(a, int) and isinstance(b, int):
            return a + b
        if isinstance(a, int):
            if a != 0:
                raise ValueError(f"adding constant {a} to a field in {self.name}")
            return b
        if isinstance(b, int):
            if b != 0:
                raise ValueError(f"adding constant {b} to a field in {self.name}")
            return a
        return a.merge(b, self.name)

    # ---- statements --------------------------------------------------------------------------
    def assign(self, target, expr, augment, env):
        target = target.strip()
        value = self.parse(self.tokenize(expr))
        if "." not in target and "[" not in target:
            # a local (`int id = ...`, `team_id = ...`, `player_id = ...`)
            name = target.split()[-1]
            if augment and name in self.locals:
                value = self.locals[name].merge(value, f"{self.name}:{name}")
            self.locals[name] = value
            if name == "id":
                self.record("id", value, augment)
            return
        path = OBJ_PREFIX.sub("", target)
        path = re.sub(r"\[(\w+)\]", lambda m: f"[{env.get(m.group(1), m.group(1))}]", path)
        self.record(path, value, augment)

    def record(self, path, value, augment):
        if isinstance(value, int):
            return  # constant assignment (e.g. skin fix-ups) carries no layout
        if augment and path in self.fields:
            self.fields[path] = self.fields[path].merge(value, f"{self.name}:{path}")
        else:
            self.fields[path] = value
            self.order.append(path)

    def string_start(self, target):
        path = OBJ_PREFIX.sub("", target.strip())
        self.pending_string = (path, self.cb)

    def advance(self, n):
        if self.pending_string is not None:
            path, start = self.pending_string
            self.strings[path] = (start, n)
            self.order.append(path)
            self.pending_string = None
        self.cb += n

    # ---- results -----------------------------------------------------------------------------
    def result(self):
        out = []
        for path in self.order:
            if path in self.strings:
                start, length = self.strings[path]
                out.append({"field": path, "kind": "string", "byte_offset": start, "len": length})
                continue
            slots = self.fields[path].slots
            live = [(i, s) for i, s in enumerate(slots) if s is not None]
            if not live:
                print(f"  WARNING {self.name}:{path} reads no bits (reference bug: masked to zero)")
                out.append({"field": path, "kind": "none"})
                continue
            width = len(live)
            if [i for i, _ in live] != list(range(width)):
                raise ValueError(f"{self.name}:{path} field bits not dense: {live}")
            first = live[0][1][0] * 8 + live[0][1][1]
            for i, (b, bit) in live:
                if b * 8 + bit != first + i:
                    raise ValueError(f"{self.name}:{path} not a contiguous LSB-first run: {live}")
            out.append({"field": path, "kind": "bits", "bit_offset": first, "width": width})
        return {"fields": out, "size": self.cb}


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return "\n".join(line.split("//")[0] for line in text.splitlines())


def function_body(source, name):
    m = re.search(rf"\b{re.escape(name)}\s*\([^)]*\)\s*\{{", source)
    if not m:
        raise KeyError(name)
    depth, i = 1, m.end()
    while depth:
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
        i += 1
    return source[m.end() : i - 1]


def split_statements(body):
    """Yield ('stmt', text) or ('block', header, inner) in order."""
    i, n = 0, len(body)
    while i < n:
        while i < n and body[i].isspace():
            i += 1
        if i >= n:
            break
        j = i
        if body.startswith("for", i):
            # the header's own `;` separators sit inside its parentheses
            depth = 0
            while True:
                if body[j] == "(":
                    depth += 1
                elif body[j] == ")":
                    depth -= 1
                    if depth == 0:
                        j += 1
                        break
                j += 1
        while j < n and body[j] not in ";{":
            j += 1
        if j >= n:
            break
        if body[j] == ";":
            yield ("stmt", body[i:j].strip())
            i = j + 1
        else:
            header = body[i:j].strip()
            depth, k = 1, j + 1
            while depth:
                if body[k] == "{":
                    depth += 1
                elif body[k] == "}":
                    depth -= 1
                k += 1
            inner = body[j + 1 : k - 1]
            i = k
            yield ("block", header, inner)


FOR = re.compile(r"for\s*\(\s*(?:int\s+)?(\w+)\s*=\s*(\w+)\s*;\s*\w+\s*<\s*([\w()\*\+ ]+?)\s*;\s*\w+\s*(?:\+\+|\+=\s*1)\s*\)")


def run(walk, body, env):
    for item in split_statements(body):
        if item[0] == "stmt":
            stmt = item[1]
            if not stmt:
                continue
            if stmt.startswith("break") or stmt.startswith("return"):
                if "current_byte" in stmt:
                    raise ValueError(f"early exit with byte movement in {walk.name}: {stmt}")
                continue
            if stmt in ("current_byte++", "++current_byte"):
                walk.advance(1); continue
            m = re.match(r"current_byte\s*\+=\s*(.+)$", stmt)
            if m:
                walk.advance(walk.eval_int(m.group(1))); continue
            if re.match(r"current_byte\s*=", stmt):
                raise ValueError(f"absolute current_byte assignment in {walk.name}: {stmt}")
            if stmt.startswith("MultiByteToWideChar") or stmt.startswith("strncpy_s"):
                args = stmt[stmt.index("(") + 1 : stmt.rindex(")")]
                parts = [p.strip() for p in re.split(r",(?![^(]*\))", args)]
                target = parts[4] if stmt.startswith("MultiByteToWideChar") else parts[0]
                walk.string_start(target); continue
            m = re.match(r"^(?:\w[\w\s]*?\s)?([\w\.\[\]]+)\s*(\+=|=)(?!=)\s*(.+)$", stmt)
            if m and "current_byte" not in m.group(1):
                target, op, expr = m.groups()
                if "data[" not in expr and "read_data" not in expr and not any(k in expr for k in walk.locals):
                    continue  # bookkeeping (b_show, num_on_team, t_ind searches)
                walk.assign(target, expr, op == "+=", env); continue
            if "current_byte" in stmt or "data[" in stmt or "read_data" in stmt:
                raise ValueError(f"unhandled statement in {walk.name}: {stmt}")
            continue
        header, inner = item[1], item[2]
        m = FOR.match(header)
        if m:
            var, start, bound = m.group(1), m.group(2), m.group(3)
            try:
                count = walk.eval_int(bound.replace("team_entry::team_max", "40"))
            except Exception:
                if "current_byte" in inner or "data[" in inner:
                    raise ValueError(f"loop with unknown bound moves bytes in {walk.name}: {header}")
                continue
            for value in range(int(env.get(start, start)), count):
                run(walk, inner, {**env, var: value})
            continue
        if header.startswith("if") or header.startswith("else"):
            if "current_byte" in inner or "read_data" in inner:
                raise ValueError(f"conditional byte movement in {walk.name}: {header}")
            continue
        raise ValueError(f"unhandled block in {walk.name}: {header}")


FUNCTIONS = {
    15: ["read_player_entry15", "read_appearance_entry15_raw", "read_team_ids15", "read_team_rosters15", "read_team_tactics15"],
    16: ["fill_player_entry16", "fill_appearance_entry16", "fill_team_ids16", "fill_team_rosters16", "fill_team_tactics16"],
    17: ["fill_player_entry17", "fill_team_ids17", "fill_team_rosters17", "fill_team_tactics17"],
    18: ["fill_player_entry18", "fill_team_ids18", "fill_team_rosters18", "fill_team_tactics18", "fill_player_entry18_texport", "fill_team_tactics18_texport"],
    19: ["fill_player_entry19", "fill_team_ids19", "fill_team_rosters19", "fill_team_tactics19", "fill_player_entry19_texport", "fill_team_tactics19_texport"],
    20: ["fill_player_entry20", "fill_team_ids20", "fill_team_ids21", "fill_team_rosters20", "fill_team_tactics20", "fill_player_entry20_texport", "fill_team_tactics20_texport"],
}



# ---- vocabularies: derived path -> (variant, doc) ----------------------------------------------
PLAYER = {
    "id": ("Id", "The player id (team id x 100 + slot for cup players)."),
    "nation": ("Nationality", "Nationality id."),
    "height": ("Height", "Height in cm."),
    "weight": ("Weight", "Weight in kg."),
    "gc1": ("GoalCelebration1", "First goal celebration."),
    "gc2": ("GoalCelebration2", "Second goal celebration."),
    "atk": ("AttackingProwess", "Attacking Prowess (Offensive Awareness)."),
    "def": ("DefensiveProwess", "Defensive Prowess (Defensive Awareness)."),
    "gk": ("Goalkeeping", "Goalkeeping (GK Awareness)."),
    "drib": ("Dribbling", "Dribbling."),
    "mo_fk": ("FreeKickMotion", "Free kick motion."),
    "finish": ("Finishing", "Finishing."),
    "lowpass": ("LowPass", "Low Pass."),
    "loftpass": ("LoftedPass", "Lofted Pass."),
    "header": ("Heading", "Header (Heading)."),
    "form": ("Form", "Form (Condition), 1 to 8 stored 0 to 7."),
    "b_edit_player": ("EditedPlayer", "The player was created or edited."),
    "swerve": ("Swerve", "Swerve (Curl)."),
    "catching": ("Catching", "Catching (Saving on PES 15, GK Catching later)."),
    "clearing": ("Clearing", "Clearing (GK Clearing)."),
    "reflex": ("Reflexes", "Reflexes (GK Reflexes)."),
    "injury": ("InjuryResistance", "Injury Resistance."),
    "b_edit_basicset": ("EditedBasicSettings", "The basic settings were edited."),
    "body_ctrl": ("BodyControl", "Body Control (Body Balance on PES 16)."),
    "phys_cont": ("PhysicalContact", "Physical Contact."),
    "kick_pwr": ("KickingPower", "Kicking Power."),
    "exp_pwr": ("ExplosivePower", "Explosive Power (Acceleration)."),
    "mo_armd": ("ArmMovementDribbling", "Arm movement while dribbling."),
    "b_edit_regpos": ("EditedRegisteredPosition", "The registered position was edited."),
    "age": ("Age", "Age."),
    "reg_pos": ("RegisteredPosition", "Registered position, GK 0 to CF 12."),
    "play_style": ("PlayingStyle", "Playing style, a version-specific index (see `convert`)."),
    "ball_ctrl": ("BallControl", "Ball Control."),
    "ball_win": ("BallWinning", "Ball Winning (Tackling)."),
    "weak_acc": ("WeakFootAccuracy", "Weak Foot Accuracy."),
    "jump": ("Jump", "Jump."),
    "mo_armr": ("ArmMovementRunning", "Arm movement while running."),
    "mo_ck": ("CornerKickMotion", "Corner kick motion."),
    "cover": ("Coverage", "Coverage (GK Coverage)."),
    "weak_use": ("WeakFootUsage", "Weak Foot Usage."),
    "play_pos[]": ("PlayablePosition", "Playable position rating for position `n` (0 none, 1 C, 2 B, 3 A), CF 0 to GK 12."),
    "mo_hunchd": ("HunchingDribbling", "Hunching while dribbling."),
    "mo_hunchr": ("HunchingRunning", "Hunching while running."),
    "mo_pk": ("PenaltyKickMotion", "Penalty kick motion."),
    "place_kick": ("PlaceKicking", "Place Kicking."),
    "b_edit_playpos": ("EditedPlayablePositions", "The playable positions were edited."),
    "b_edit_ability": ("EditedAbilities", "The abilities were edited."),
    "b_edit_skill": ("EditedSkills", "The skills were edited."),
    "stamina": ("Stamina", "Stamina."),
    "speed": ("Speed", "Speed."),
    "b_edit_style": ("EditedPlayingStyle", "The playing style was edited."),
    "b_edit_com": ("EditedComStyles", "The COM playing styles were edited."),
    "b_edit_motion": ("EditedMotion", "The motions were edited."),
    "b_base_copy": ("BaseCopy", "The player is a base copy of another."),
    "strong_foot": ("StrongerFoot", "Stronger foot (0 right, 1 left)."),
    "com_style[]": ("ComStyle", "COM playing style `n` (0 Trickster to 6 Long Ranger)."),
    "play_skill[]": ("Skill", "Player skill `n` (0 Scissors Feint to 40 Through Passing)."),
    "star": ("Star", "Star rating (PES 19+)."),
    "mo_drib": ("DribblingMotion", "Dribbling motion (PES 20+)."),
    "tight_pos": ("TightPossession", "Tight Possession (PES 20+)."),
    "aggres": ("Aggression", "Aggression (PES 20+)."),
    "play_attit": ("PlayingAttitude", "Playing attitude (PES 20+)."),
    "strong_hand": ("StrongerHand", "Stronger hand (PES 20+)."),
    "appearance_id": ("AppearanceId", "The id of the player whose appearance the block holds: the player's own, or -1 (all bits set) on a stripped Fox save, where the database tables decide the look."),
    "b_edit_face": ("EditedFace", "The face was edited."),
    "b_edit_hair": ("EditedHair", "The hairstyle was edited."),
    "b_edit_phys": ("EditedPhysique", "The physique was edited."),
    "b_edit_strip": ("EditedStrip", "The strip style was edited."),
    "boot_id": ("BootsId", "Boots model id."),
    "glove_id": ("GlovesId", "Gloves model id (any player; 0 = normal hands)."),
    "copy_id": ("BaseCopyId", "Base copy player id (the player's own id when unset)."),
    "neck_len": ("NeckLength", "Neck length."),
    "neck_size": ("NeckSize", "Neck size."),
    "shldr_hi": ("ShoulderHeight", "Shoulder height."),
    "shldr_wid": ("ShoulderWidth", "Shoulder width."),
    "chest": ("Chest", "Chest measurement."),
    "waist": ("Waist", "Waist size."),
    "arm_size": ("ArmSize", "Arm size."),
    "arm_len": ("ArmLength", "Arm length."),
    "thigh": ("Thigh", "Thigh size."),
    "calf": ("Calf", "Calf size."),
    "leg_len": ("LegLength", "Leg length."),
    "head_len": ("HeadLength", "Head length."),
    "head_wid": ("HeadWidth", "Head width."),
    "head_dep": ("HeadDepth", "Head depth."),
    "wrist_col_l": ("WristTapeColorLeft", "Left wrist tape colour."),
    "wrist_col_r": ("WristTapeColorRight", "Right wrist tape colour."),
    "wrist_tape": ("WristTaping", "Wrist taping."),
    "spec_col": ("SpectaclesColor", "Spectacles frame colour."),
    "spec_style": ("SpectaclesStyle", "Spectacles style."),
    "sleeve": ("Sleeves", "Sleeves."),
    "inners": ("Inners", "Long-sleeved inners."),
    "socks": ("Socks", "Sock length."),
    "undershorts": ("Undershorts", "Undershorts."),
    "untucked": ("Untucked", "Shirttail out."),
    "ankle_tape": ("AnkleTaping", "Ankle taping."),
}
# The fields the ingame-face run carries (`schema/ingame_face.rs`): they leave the per-version
# tables and the `PlayerField` enum for `IngameFace` accessors, so no byte has two owners.
INGAME_FACE_PATHS = ("gloves", "gloves_col", "skin_col", "iris_col")
# The measured run, byte 22 of the appearance block to the block's end
# (`spec/pes_savefile/model.md` "The ingame-face run"): the 15/16 appearance record, the
# player record on 17+.
EXPECTED_RUN = {15: (22, 46), 16: (22, 50), 17: (138, 50), 18: (138, 50), 19: (138, 50), 20: (262, 50)}
# Fields the reference itself under-reads, corrected to the stored width the save
# census measured: PES 18 records set bit 297 for 2,808 players where the walk reads
# only 2 of mo_armd's bits.
WIDTH_FIXES = {(18, "mo_armd"): 3}
# Fields the reference never reads, measured in-game, as byte offsets.
# "man_marking": the first byte of
# each preset's eleven-byte man-marking run within a preset block (byte i is own
# formation slot i's assignment: the marked opponent's formation slot, 0xFF
# none); the three presets sit one preset_stride apart, and PES 21 shares PES
# 20's tables. "auto_flags": PES 15's three auto-flag bytes, in the order PES
# 16's derived walk reads them. "team_ids": (field, byte, bit width) of the team
# record ids the reference reads from PES 19 on at most. PES 17 and 18 keep them at
# PES 19's offsets: on the reference saves the u32 at +4 equals the team id on 218
# of 220 PES 17 teams and 200 of 202 PES 18 ones, and the u16 at +10 equals the
# same team's PES 19 stadium on 198 of 210 and 168 of 202. PES 15 and 16 lack the
# manager id's four bytes and keep the stadium at +6: it equals PES 19's on 198 of
# 210 PES 15 teams and 158 of 192 PES 16 ones (and PES 17's on 166 of 192).
MEASURED_FIELDS = {
    "man_marking": {15: 0x73, 16: 0x73, 17: 0x95, 18: 0x95, 19: 0x95, 20: 0x95},
    "auto_flags": {
        15: [
            ("auto_substitution", 0x1FE),
            ("auto_offside_trap", 0x1FF),
            ("auto_preset_change", 0x200),
        ],
    },
    "team_ids": {
        15: [("stadium_id", 6, 16)],
        16: [("stadium_id", 6, 16)],
        17: [("manager_id", 4, 32), ("stadium_id", 10, 16)],
        18: [("manager_id", 4, 32), ("stadium_id", 10, 16)],
    },
}
PLAYER_TEXT = {"name": ("Name", "The player name, UTF-8, colour codes included."),
               "shirt_name": ("ShirtName", "The shirt name, single-byte text.")}
TEAM = {
    "id": ("Id", "The team id."),
    "manager_id": ("ManagerId", "Manager id (PES 17+)."),
    "stadium_id": ("StadiumId", "Home stadium id."),
    "b_edit_name": ("EditedName", "The team name was edited."),
    "b_edit_shortname": ("EditedShortName", "The short name was edited (PES 15)."),
    "b_edit_stadium": ("EditedStadium", "The home stadium was edited (PES 20+)."),
    "b_edit_strip": ("EditedStrip", "The strips were edited (PES 17)."),
    "color1_red": ("Color1Red", "First team colour, red, 6 bits."),
    "color1_green": ("Color1Green", "First team colour, green, 6 bits."),
    "color1_blue": ("Color1Blue", "First team colour, blue, 6 bits."),
    "color2_red": ("Color2Red", "Second team colour, red, 6 bits."),
    "color2_green": ("Color2Green", "Second team colour, green, 6 bits."),
    "color2_blue": ("Color2Blue", "Second team colour, blue, 6 bits."),
    "stripBlock[].stripNumber": ("KitSlotNumber", "Kit slot `n`: kit number from 0, or 0x80 for a goalkeeper kit."),
    "stripBlock[].stripTeamId": ("KitSlotTeam", "Kit slot `n`: the bound team id x 0x40."),
}
TEAM_TEXT = {"name": ("Name", "The team name, UTF-8."), "short_name": ("ShortName", "The three-letter short name.")}
ROSTER = {"team_id": ("TeamId", "The team id the roster belongs to."),
          "players[]": ("Player", "Roster slot `n`: player id, 0 when empty."),
          "numbers[]": ("Number", "Roster slot `n`: shirt number.")}
TACTICS_SCALAR = {
    "team_id": ("TeamId", "The team id the tactics belong to."),
    "fk_taker_long": ("FreeKickTakerLong", "Long free kick taker (roster slot, 0xFF none)."),
    "fk_taker_short": ("FreeKickTakerShort", "Short free kick taker."),
    "fk_taker_2": ("FreeKickTakerSecond", "Second free kick taker."),
    "ck_taker_left": ("CornerTakerLeft", "Left corner taker."),
    "ck_taker_right": ("CornerTakerRight", "Right corner taker."),
    "pk_taker": ("PenaltyTaker", "Penalty taker."),
    "captain_ind": ("Captain", "Captain (roster slot)."),
    "auto_substitution": ("AutoSubstitution", "Auto substitution setting."),
    "auto_offside_trap": ("AutoOffsideTrap", "Auto offside trap."),
    "auto_preset_change": ("AutoPresetChange", "Auto preset change."),
    "auto_change_atk_def_levels": ("AutoAttackDefenceLevels", "Auto attack/defence level change."),
    "starting11[]": ("Starting", "Starting eleven position `n`: roster slot."),
    "bench_order[]": ("Bench", "Bench order entry `n`: roster slot."),
    "players_to_join_attack[]": ("PlayerToJoinAttack", "Player `n` joining the attack."),
}
PRESET = {
    "attacking_style": ("AttackingStyle", "0 counter attack, 1 possession."),
    "buildup": ("Buildup", "0 long pass, 1 short pass."),
    "attacking_zone": ("AttackingZone", "0 centre, 1 wide."),
    "positioning": ("Positioning", "0 maintain, 1 flexible."),
    "defensive_style": ("DefensiveStyle", "0 frontline pressure, 1 all-out defence."),
    "containment_area": ("ContainmentArea", "0 middle, 1 wide."),
    "pressure": ("Pressure", "0 aggressive, 1 conservative."),
    "fluid": ("FluidFormation", "Fluid formation on/off (PES 16+)."),
    "support_range": ("SupportRange", "Support range, 1 to 10."),
    "defensive_line": ("DefensiveLine", "Defensive line, 1 to 10."),
    "compactness": ("Compactness", "Compactness, 1 to 10."),
    "numbers_in_attack": ("NumbersInAttack", "Numbers in attack, 1 few to 3 many."),
    "numbers_in_defense": ("NumbersInDefence", "Numbers in defence, 1 few to 3 many."),
}

INDEXED = re.compile(r"^(.*)\[(\d+)\](.*)$")


def emit_enum(out, name, doc, entries):
    out.append(f"/// {doc}")
    out.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
    out.append(f"pub enum {name} {{")
    for variant, vdoc, payload in entries:
        out.append(f"    /// {vdoc}")
        out.append(f"    {variant}{payload},")
    out.append("}")
    out.append("")


def fields_rs():
    out = ["//! The version-neutral field vocabularies the schema tables name and the codec fills.",
           "//! Indexed variants carry the element index (`Skill(3)`); the tables say where each",
           "//! version stores it.", ""]
    entries = []
    for path, (variant, doc) in PLAYER.items():
        entries.append((variant, doc, "(u8)" if "[]" in path else ""))
    emit_enum(out, "PlayerField", "A bit-run field of the player record (the appearance record on PES 15/16).", entries)
    emit_enum(out, "PlayerText", "A text field of the player record.", [(v, d, "") for v, d in PLAYER_TEXT.values()])
    entries = [(v, d, "(u8)" if "[]" in p else "") for p, (v, d) in TEAM.items()]
    emit_enum(out, "TeamField", "A bit-run field of the team record.", entries)
    emit_enum(out, "TeamText", "A text field of the team record.", [(v, d, "") for v, d in TEAM_TEXT.values()])
    entries = [(v, d, "(u8)" if "[]" in p else "") for p, (v, d) in ROSTER.items()]
    emit_enum(out, "RosterField", "A field of the roster record.", entries)
    entries = [(v, d, "(u8)" if "[]" in p else "") for p, (v, d) in TACTICS_SCALAR.items()]
    preset_doc = "/// The preset (0 to 2).\n        preset: u8,"
    entries.insert(1, ("Preset", "Per-preset setting `field` of preset `preset` (0 to 2).",
                       " {\n        " + preset_doc + "\n        /// The setting within the preset.\n        field: PresetField,\n    }"))
    entries.insert(2, ("Formation", "Formation `formation` (0 kick-off, 1 in possession, 2 out of possession) of preset `preset`, player `slot` (0 to 10).",
                       " {\n        " + preset_doc + "\n        /// The formation (0 kick-off, 1 in possession, 2 out of possession).\n        formation: u8,"
                       "\n        /// The player slot (0 to 10).\n        slot: u8,\n        /// Which value of the slot.\n        part: FormationPart,\n    }"))
    entries.insert(3, ("Instruction", "Advanced instruction `index` (0 or 1) on `side` of preset `preset`.",
                       " {\n        " + preset_doc + "\n        /// Attacking or defending instruction.\n        side: InstructionSide,"
                       "\n        /// The instruction index (0 or 1).\n        index: u8,\n        /// Which value of the instruction.\n        part: InstructionPart,\n    }"))
    entries.insert(4, ("ManMarking", "Man-marking assignment of own formation slot `slot` in preset `preset`: the marked opponent's formation slot, 0xFF none.",
                       " {\n        " + preset_doc + "\n        /// The own formation slot (0 to 10).\n        slot: u8,\n    }"))
    emit_enum(out, "TacticsField", "A field of the tactics record.", entries)
    emit_enum(out, "PresetField", "A per-preset tactics setting.", [(v, d, "") for v, d in PRESET.values()])
    emit_enum(out, "FormationPart", "Which value of a formation slot.", [("Position", "Position, GK 0 to CF 12.", ""), ("X", "Horizontal coordinate.", ""), ("Y", "Vertical coordinate.", "")])
    emit_enum(out, "InstructionSide", "Attacking or defending instruction.", [("Attack", "An attacking instruction.", ""), ("Defence", "A defending instruction.", "")])
    emit_enum(out, "InstructionPart", "Which value of an advanced instruction.", [("Instruction", "The instruction id (version-specific meaning).", ""), ("PlayerId", "The targeted player, for instructions that take one.", "")])
    return "\n".join(out).rstrip() + "\n"


def split_rows(fields, vocab, texts_vocab):
    """Group a walk's fields into scalar rows, indexed groups and texts."""
    scalars, groups, texts = [], defaultdict(list), []
    for f in fields:
        if f["kind"] == "none":
            continue
        if f["kind"] == "string":
            texts.append((texts_vocab[f["field"]][0], f["byte_offset"], f["len"]))
            continue
        m = INDEXED.match(f["field"])
        if m and m.group(3) == "":
            base = f"{m.group(1)}[]{m.group(3)}"
            groups[base].append((int(m.group(2)), f["bit_offset"], f["width"]))
        elif m:
            base = f"{m.group(1)}[]{m.group(3)}"
            groups[base].append((int(m.group(2)), f["bit_offset"], f["width"]))
        else:
            scalars.append((vocab[f["field"]][0], f["bit_offset"], f["width"]))
    arrays, loose = [], []
    for base, items in groups.items():
        items.sort()
        variant = vocab[base][0]
        indices = [i for i, _, _ in items]
        widths = {w for _, _, w in items}
        regular = (indices == list(range(len(items))) and len(widths) == 1 and len(items) > 1
                   and len({items[i + 1][1] - items[i][1] for i in range(len(items) - 1)}) == 1)
        if regular:
            arrays.append((variant, len(items), items[0][1], items[1][1] - items[0][1], items[0][2]))
        else:
            loose.extend((f"{variant}({i})", off, w) for i, off, w in items)
    return scalars + loose, arrays, texts


def appearance_block(fields):
    """The appearance block's first byte, or None when the record carries no appearance
    block. The block start comes from the BootsId row: boots/gloves is the block's second
    u32."""
    if not any(f["field"] in INGAME_FACE_PATHS for f in fields):
        return None
    boots = next(f for f in fields if f["field"] == "boot_id")
    return boots["bit_offset"] // 8 - 4


def ingame_face_run(fields, size, expected):
    """The byte run from the appearance block's 22nd byte to its end (the block fills the rest
    of the record), or None when the record carries no appearance block."""
    block = appearance_block(fields)
    if block is None:
        return None
    run = (block + 22, size - (block + 22))
    assert run == expected, f"ingame-face run {run}, expected {expected}"
    return run


def record_rs(name, field_ty, text_ty, size, scalars, arrays, texts, doc, ingame_face=None):
    out = [f"/// {doc}", f"pub(crate) static {name}: RecordSchema<{field_ty}, {text_ty}> = RecordSchema {{",
           f"    size: {size},", "    fields: &["]
    for variant, off, w in scalars:
        out.append(f"        FieldSpec {{ field: {field_ty}::{variant}, bit_offset: {off}, bit_width: {w} }},")
    out.append("    ],")
    out.append("    arrays: &[")
    for variant, count, base, stride, w in arrays:
        out.append(f"        ArraySpec {{ make: {field_ty}::{variant}, count: {count}, base_bit: {base}, stride_bits: {stride}, bit_width: {w} }},")
    out.append("    ],")
    out.append("    texts: &[")
    for variant, off, ln in texts:
        out.append(f"        TextSpec {{ text: {text_ty}::{variant}, byte_offset: {off}, len: {ln} }},")
    out.append("    ],")
    if ingame_face:
        out.append(f"    ingame_face: Some(ByteRun {{ byte_offset: {ingame_face[0]}, len: {ingame_face[1]} }}),")
    else:
        out.append("    ingame_face: None,")
    out.append("};")
    out.append("")
    return out


def tactics_rs(name, rec, doc, man_marking_bit):
    fields = {f["field"]: f for f in rec["fields"] if f["kind"] != "none"}

    def off(path):
        return fields[path]["bit_offset"]
    base = off("presets[0].formations[0].players[0].pos")
    pstride = off("presets[1].formations[0].players[0].pos") - base
    fstride = off("presets[0].formations[1].players[0].pos") - base
    y0 = off("presets[0].formations[0].players[0].y") - base
    x0 = off("presets[0].formations[0].players[0].x") - base
    slot_stride = off("presets[0].formations[0].players[1].pos") - base
    pair_stride = off("presets[0].formations[0].players[1].y") - base - y0
    for p in range(3):
        for f in range(3):
            for k in range(11):
                b = base + p * pstride + f * fstride
                assert off(f"presets[{p}].formations[{f}].players[{k}].pos") == b + k * slot_stride
                assert off(f"presets[{p}].formations[{f}].players[{k}].y") == b + y0 + k * pair_stride
                assert off(f"presets[{p}].formations[{f}].players[{k}].x") == b + x0 + k * pair_stride
                for part in ("pos", "x", "y"):
                    assert fields[f"presets[{p}].formations[{f}].players[{k}].{part}"]["width"] == 8
    preset_rows = []
    for cpp, (variant, _) in PRESET.items():
        if f"presets[0].{cpp}" not in fields:
            continue
        o = [off(f"presets[{p}].{cpp}") for p in range(3)]
        assert o[1] - o[0] == pstride and o[2] - o[1] == pstride, cpp
        assert all(fields[f"presets[{p}].{cpp}"]["width"] == 8 for p in range(3))
        preset_rows.append((variant, o[0]))
    instructions = None
    if "presets[0].atk_instructions[0].instruction" in fields:
        ib = off("presets[0].atk_instructions[0].instruction")
        side = off("presets[0].def_instructions[0].instruction") - ib
        index = off("presets[0].atk_instructions[1].instruction") - ib
        part = off("presets[0].atk_instructions[0].player_id") - ib
        for p in range(3):
            for s, sn in enumerate(("atk", "def")):
                for i in range(2):
                    for q, qn in enumerate(("instruction", "player_id")):
                        path = f"presets[{p}].{sn}_instructions[{i}].{qn}"
                        assert off(path) == ib + p * pstride + s * side + i * index + q * part, path
                        assert fields[path]["width"] == 8
        instructions = (ib, side, index, part)
    scalar_fields = [f for f in rec["fields"] if f["kind"] != "none" and not f["field"].startswith("presets")]
    scalars, arrays, texts = split_rows(scalar_fields, TACTICS_SCALAR, {})
    assert not texts
    out = [f"/// {doc}", f"pub(crate) static {name}: TacticsSchema = TacticsSchema {{",
           f"    size: {rec['size']},", "    fields: &["]
    for variant, o, w in scalars:
        out.append(f"        FieldSpec {{ field: TacticsField::{variant}, bit_offset: {o}, bit_width: {w} }},")
    out.append("    ],")
    out.append("    arrays: &[")
    for variant, count, b, stride, w in arrays:
        out.append(f"        ArraySpec {{ make: TacticsField::{variant}, count: {count}, base_bit: {b}, stride_bits: {stride}, bit_width: {w} }},")
    out.append("    ],")
    out.append(f"    preset_stride_bits: {pstride},")
    out.append(f"    man_marking_bit: {man_marking_bit},")
    out.append("    presets: &[")
    for variant, o in preset_rows:
        out.append(f"        PresetSpec {{ field: PresetField::{variant}, bit_offset: {o} }},")
    out.append("    ],")
    out.append("    formations: FormationLayout {")
    out.append(f"        base_bit: {base},")
    out.append(f"        formation_stride_bits: {fstride},")
    out.append(f"        slot_stride_bits: {slot_stride},")
    out.append(f"        y_offset_bits: {y0},")
    out.append(f"        x_offset_bits: {x0},")
    out.append(f"        pair_stride_bits: {pair_stride},")
    out.append("    },")
    if instructions:
        ib, side, index, part = instructions
        out.append("    instructions: Some(InstructionLayout {")
        out.append(f"        base_bit: {ib},")
        out.append(f"        side_stride_bits: {side},")
        out.append(f"        index_stride_bits: {index},")
        out.append(f"        part_stride_bits: {part},")
        out.append("    }),")
    else:
        out.append("    instructions: None,")
    out.append("};")
    out.append("")
    return out


# version: (player fn, appearance fn, team fn(s), roster fn, tactics fn, section layouts)
VERSIONS = {
    15: dict(player="read_player_entry15", appearance="read_appearance_entry15_raw", team="read_team_ids15",
             roster="read_team_rosters15", tactics="read_team_tactics15",
             layouts={"Pes15": dict(players=(0x4C, 0x34), appearance=(0x2AB9CC, 0x36), teams=(0x44AA6C, 0x38), rosters=(0x4E45CC, 0x38), tactics=(0x507194, 0x38))}),
    16: dict(player="fill_player_entry16", appearance="fill_appearance_entry16", team="fill_team_ids16",
             roster="fill_team_rosters16", tactics="fill_team_tactics16",
             layouts={"Pes16": dict(players=(0x4C, 0x34), appearance=(0x2AB9CC, 0x36), teams=(0x46310C, 0x38), rosters=(0x4FCC6C, 0x38), tactics=(0x51F814, 0x38))}),
    17: dict(player="fill_player_entry17", team="fill_team_ids17", roster="fill_team_rosters17", tactics="fill_team_tactics17",
             layouts={"Pes17": dict(players=(0x78, 0x5C), teams=(0x3C3E58, 0x60), coaches=(0x410118, 0x62), rosters=(0x475A90, 0x60), tactics=(0x490640, 0x60))}),
    18: dict(player="fill_player_entry18", team="fill_team_ids18", roster="fill_team_rosters18", tactics="fill_team_tactics18",
             layouts={"Pes18": dict(players=(0x7C, 0x60), teams=(0x3C3E5C, 0x64), coaches=(0x40A35C, 0x66), rosters=(0x46FF54, 0x64), tactics=(0x488B74, 0x64))}),
    19: dict(player="fill_player_entry19", team="fill_team_ids19", roster="fill_team_rosters19", tactics="fill_team_tactics19",
             layouts={"Pes19": dict(players=(0x7C, 0x60), teams=(0x5BCC7C, 0x64), coaches=(0x5FECBC, 0x66), rosters=(0x6773C4, 0x64), tactics=(0x69EC8C, 0x64))}),
    20: dict(player="fill_player_entry20", team="fill_team_ids20", team21="fill_team_ids21", roster="fill_team_rosters20", tactics="fill_team_tactics20",
             layouts={"Pes20": dict(players=(0x7C, 0x60), teams=(0x8ED2FC, 0x64), coaches=(0x94DDDC, 0x66), rosters=(0x9CCC04, 0x64), tactics=(0xA01E3C, 0x64)),
                      "Pes21": dict(players=(0x7C, 0x60), teams=(0x8ED2FC, 0x64), coaches=(0x958DA4, 0x66), rosters=(0x9D4648, 0x64), tactics=(0xA09880, 0x64))}),
}


def insert_by_offset(fields, row):
    """`fields` with an injected bit-run `row` before the first bit run that starts after it,
    so the emitted table keeps the walk's order."""
    at = next(
        i
        for i, f in enumerate(fields)
        if f["kind"] == "bits" and f["bit_offset"] > row["bit_offset"]
    )
    return fields[:at] + [row] + fields[at:]


def version_rs(version, spec, walks):
    label = "PES 20 and 21" if version == 20 else f"PES {version}"
    out = [f"//! The {label} field tables, derived mechanically (method and checks:",
           "//! `docs/spec/pes_savefile/codec.md`, \"Schema-driven save codec\"). Generated by",
           "//! `scripts/derive_savefile_schema.py`; do not edit by hand.", "",
           "use pes_version::PesVersion;", ""]
    body = []
    rec = walks[spec["player"]]
    for f in rec["fields"]:
        if f["kind"] == "bits":
            f["width"] = WIDTH_FIXES.get((version, f["field"]), f["width"])
    run = ingame_face_run(rec["fields"], rec["size"], EXPECTED_RUN[version])
    fields = [f for f in rec["fields"] if f["field"] not in INGAME_FACE_PATHS]
    block = appearance_block(rec["fields"])
    if block is not None:
        # The block's first u32 is the appearance id: the reference reads past it, so the
        # row is injected like the 15/16 appearance record's `id` (which is the join key).
        row = {"field": "appearance_id", "kind": "bits", "bit_offset": block * 8, "width": 32}
        fields = insert_by_offset(fields, row)
    s, a, t = split_rows(fields, PLAYER, PLAYER_TEXT)
    body += record_rs("PLAYER", "PlayerField", "PlayerText", rec["size"], s, a, t,
                      f"The {label} player record.", ingame_face=run)
    if "appearance" in spec:
        rec = walks[spec["appearance"]]
        run = ingame_face_run(rec["fields"], rec["size"], EXPECTED_RUN[version])
        fields = [{"field": "id", "kind": "bits", "bit_offset": 0, "width": 32}
                  ] + [f for f in rec["fields"] if f["field"] not in INGAME_FACE_PATHS]
        s, a, t = split_rows(fields, PLAYER, PLAYER_TEXT)
        body += record_rs("APPEARANCE", "PlayerField", "PlayerText", rec["size"], s, a, t,
                          f"The {label} appearance record; the player id at +0 keys it to its player record.",
                          ingame_face=run)
    rec = walks[spec["team"]]
    fields = rec["fields"]
    for name, byte, width in MEASURED_FIELDS["team_ids"].get(version, []):
        fields = insert_by_offset(fields, {"field": name, "kind": "bits", "bit_offset": byte * 8, "width": width})
    s, a, t = split_rows(fields, TEAM, TEAM_TEXT)
    body += record_rs("TEAM", "TeamField", "TeamText", rec["size"], s, a, t, f"The {label.split(' and')[0]} team record.")
    if "team21" in spec:
        rec = walks[spec["team21"]]
        s, a, t = split_rows(rec["fields"], TEAM, TEAM_TEXT)
        body += record_rs("TEAM_21", "TeamField", "TeamText", rec["size"], s, a, t, "The PES 21 team record.")
    rec = walks[spec["roster"]]
    fields = [{"field": "team_id", "kind": "bits", "bit_offset": 0, "width": 32}] + [f for f in rec["fields"]]
    s, a, t = split_rows(fields, ROSTER, {})
    body += record_rs("ROSTER", "RosterField", "TeamText", rec["size"], s, a, t, f"The {label} roster record.")
    rec = walks[spec["tactics"]]
    rec = dict(rec, fields=[{"field": "team_id", "kind": "bits", "bit_offset": 0, "width": 32}] + rec["fields"])
    rec["fields"] += [
        {"field": name, "kind": "bits", "bit_offset": byte * 8, "width": 8}
        for name, byte in MEASURED_FIELDS["auto_flags"].get(version, [])
    ]
    body += tactics_rs("TACTICS", rec, f"The {label} tactics record.",
                       MEASURED_FIELDS["man_marking"][version] * 8)
    for name, lay in spec["layouts"].items():
        body.append(f"/// Where every section sits in the {name[:3]} {name[3:]} payload.")
        body.append(f"pub(crate) static {name.upper()}: VersionSchema = VersionSchema {{")
        body.append(f"    version: PesVersion::{name},")
        body.append(f"    players: SectionLayout {{ offset: {lay['players'][0]:#x}, count_offset: {lay['players'][1]:#x} }},")
        body.append("    player: &PLAYER,")
        if "appearance" in lay:
            body.append(f"    appearance: Some((SectionLayout {{ offset: {lay['appearance'][0]:#x}, count_offset: {lay['appearance'][1]:#x} }}, &APPEARANCE)),")
        else:
            body.append("    appearance: None,")
        body.append(f"    teams: SectionLayout {{ offset: {lay['teams'][0]:#x}, count_offset: {lay['teams'][1]:#x} }},")
        body.append(f"    team: &{'TEAM_21' if name == 'Pes21' else 'TEAM'},")
        if "coaches" in lay:
            body.append(f"    coaches: Some(SectionLayout {{ offset: {lay['coaches'][0]:#x}, count_offset: {lay['coaches'][1]:#x} }}),")
        else:
            body.append("    coaches: None,")
        body.append(f"    rosters: SectionLayout {{ offset: {lay['rosters'][0]:#x}, count_offset: {lay['rosters'][1]:#x} }},")
        body.append("    roster: &ROSTER,")
        body.append(f"    tactics: SectionLayout {{ offset: {lay['tactics'][0]:#x}, count_offset: {lay['tactics'][1]:#x} }},")
        body.append("    tactic: &TACTICS,")
        body.append("};")
        body.append("")
    text = "\n".join(body)
    used_fields = [n for n in ("FormationPart", "InstructionPart", "InstructionSide", "PlayerField", "PlayerText", "PresetField", "RosterField", "TacticsField", "TeamField", "TeamText") if re.search(rf"\b{n}\b", text)]
    used_types = [n for n in ("ArraySpec", "ByteRun", "FieldSpec", "FormationLayout", "InstructionLayout", "PresetSpec", "RecordSchema", "SectionLayout", "TacticsSchema", "TextSpec", "VersionSchema") if re.search(rf"\b{n}\b", text)]
    out.append("use super::fields::{" + ", ".join(used_fields) + "};")
    out.append("use super::{" + ", ".join(used_types) + "};")
    out.append("")
    return "\n".join(out) + "\n" + text.rstrip() + "\n"


def write(path, text):
    """Write atomically, then rustfmt in place (the toolchain's edition), so the committed tables
    are exactly what a regeneration produces."""
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(text, encoding="utf-8", newline="\n")
    tmp.replace(path)
    subprocess.run(["rustfmt", "--edition", "2024", str(path)], check=True)


def main():
    root = Path(sys.argv[1])
    out_dir = Path(sys.argv[2])
    derived = {}
    for version, names in FUNCTIONS.items():
        source = strip_comments((root / f"pes{version}.cpp").read_text(encoding="utf-8", errors="replace"))
        derived[str(version)] = {}
        for name in names:
            walk = Walk(source, name)
            run(walk, function_body(source, name), {})
            derived[str(version)][name] = walk.result()
            print(f"{version} {name}: {len(walk.result()['fields'])} fields, size {walk.cb}")
    out_dir.mkdir(parents=True, exist_ok=True)
    write(out_dir / "fields.rs", fields_rs())
    for version, spec in VERSIONS.items():
        write(out_dir / f"pes{version}.rs", version_rs(version, spec, derived[str(version)]))
        print(f"wrote pes{version}.rs")


main()
