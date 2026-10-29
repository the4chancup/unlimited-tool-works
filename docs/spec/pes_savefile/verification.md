# Unlimited Tool Works — Savefile plan: Verification

Part of the [Savefile plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Verification

- **Container roundtrips**: decrypt → re-encrypt → decrypt again must reproduce
  the payload byte-for-byte for every version (encryption uses a random salt, so
  ciphertexts differ; plaintexts must not). Known-answer tests for the MT19937
  keystream against the Python reference.
- **Codec roundtrips**: read → write → byte-compare for every player and team
  block in real saves of all seven versions (`Midcupping/EDIT.bin` and friends
  are test fixtures; the user's KONAMI `eFootball PES 2021 SEASON UPDATE` save
  is the 20/21 fixture — sanity anchors already verified there: player count
  u16 at 0x60, and every record's appearance playerID at +240 equals its player
  ID at +0).
- **Name fields**: names read as null-terminated UTF-8 (the `♂` in a real PES 19
  name is the bytes `E2 99 82`, colour codes are the ASCII bytes `11 63` + hex)
  from real PES 19 (`PRO EVOLUTION SOCCER 2019` KONAMI save) and PES 21 cup
  saves through the AET compiler Red's `savefile.py` reader, which shares this
  plan's offsets; `display_name()` must strip every colour code in those saves'
  ~30 decorated names and leave every undecorated name unchanged. A text that fills its field
  (no NUL) reads whole and writes back byte-identical; one byte more is refused; a non-UTF-8
  name is refused on read (`codec.md` "Text fields hold `len` bytes").
- **Export settings versus save interchange**: generated `settings.toml` carries a boots/gloves
  ID only when the stored ID is 1 to 100, never 0 or a custom ID above 100; an authored ID above
  100 is `OutOfRange` naming the key; full Team TOML still round-trips every stored ID as
  player-record data. Compiler ID assignment remains independent of the authorable settings
  serializer.
- **Settings completeness**: for every version, each player field in the schema tables is
  marked `settings`, `compiler_owned` or `gameplay` (an `Ownership` match with no wildcard
  arm), the `settings` set equals `PlayerSettings`' fields, and every bit of the appearance
  block has exactly one carrier — a `PlayerField` row, an `INGAME_FACE_FIELDS` row, or an
  `UNUSED_FACE_BITS` row — with none of it `gameplay`; the generated template contains
  every `PlayerSettings` key (a commented line for each unset
  one; `boots_id`/`gloves_id` as `""`, which parses back to unset).
- **Cross-implementation parity**: field values must match 4ccEditor's for the same save. The
  evidence is 4ccEditor's own `.4ccs` export of one team per version, taken by the maintainer
  from the save each payload fixture holds (`tests/fixtures/pesNN_parity.4ccs`): read against
  that team, every player equals the codec's read of it, `ingame_face` and the face-type keys
  excepted (the record does not carry them). Aesthetics fingerprints must match
  `compare-saves-*.py` output; the appearance block must be the slice
  `transplant-aesthetics-*.py` rewrites.
- **Cross-version conversion**: compare against the converters' verified behavior on real team
  data, excluding their documented stale PES20/21 offsets. Assert corrected name and appearance
  fields independently against the target schema; reproducing the known converter corruption is
  not parity.
- **Interchange formats**: Team TOML `from_team` → `to_toml` → `parse` → `apply` onto a copy of
  the same team reproduces the team and its players field for field on every fixture save, and
  `to_toml` of the parsed document is the emitted text again; a partial file touches only the
  fields it carries (a one-key file leaves everything else byte-identical after `write_player`);
  `.4ccs` decodes the real PES 19 fixture file's 23 players, every mapped field, against the
  ctypes offsets held as literals in the golden test (the fields PES 19's schema lacks are a
  literal list and must be absent); a `.4ccs` read against a team whose roster order differs from
  its record order puts each record on its own player's slot; the `.4cct` fixture's tactics equal the schema codec's for
  the same team; Texport `from_bytes` → `to_bytes` is byte-identical on the real 16, 17, 18, 19,
  20 and 21 fixtures and the whole tactics and every player record equal what the schema codec reads at
  the golden's literal offsets; `apply` refuses, before any write, a value wider than the target
  version's field, a text the target's field cannot encode or hold, and a NUL; the 18/19/20/21
  key indices and layouts are measured, the 16/17 payload offsets measured on 46 real files and
  PES 15's taken as 16's, unverified.
- **Patch equivalence**: compiling a fixture export against a save, and compiling it without a
  save then applying the produced patch to a copy of that save, yield byte-identical savefiles.
  Held by construction: the compiler has no second savefile write path, so the local apply and a
  later `apply-patch` of the written file are the same `AestheticsPatch::apply` over the same
  patch (`team_compiler/src/output/savefile.rs`).
