# Unlimited Tool Works — Music player plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The music player (`crates/tools/music_player`) is the successor to **Rigdio**, the
match-day goalhorn/anthem/chant player used by streamers during cup events. The
community name stays in the user-facing labels: the sidebar shows
**"Music player (Rigdio)"** (the icon rail shows the tool icon — the existing
`rigdio.ico` artwork can be reused); the crate (`music_player`) and tool id
(`music-player`) use the suite's descriptive naming.

This plan also specifies the two music lib crates, since the player is their primary
consumer: **`music_export`** (the `.4ccm` format and condition model) and
**`audio_engine`** (playback + loudness analysis). Both are shared with the
[Music export editor](music_export_editor.md) (the RigDJ successor). Platform context
is in the [core plan](core/README.md).

Unlike the rest of the suite, the music tools operate on audio files rather than game
files. They share only the platform (`studio_core`) and the two music libs — no
dependency on the format parsers, the pipeline, or the savefile crates. One further
optional dependency: when the [Match tracker](match_tracker/README.md) is running, the player
subscribes to its `match_feed` and can run the match **autonomously** — see
"Autopilot" below.

---

## Background

Rigdio (`Tools_4cc/rigdio`) is a Python/tkinter application that loads two `.4ccm`
music exports (home and away team) and presents a soundboard: an anthem button, victory
anthem buttons (including selectable "special" victory anthems), one goalhorn button
per player driven by a priority-ordered condition system, and a chants subsystem with
timed random playback. It tracks the match score (with undo), normalizes loudness
across all loaded tracks, and writes the current song title to a log file for OBS
overlays.

| Tool | Role | Location |
|------|------|----------|
| Rigdio | Match-day music player (streamer side) | `Tools_4cc/rigdio` |
| RigDJ | `.4ccm` GUI editor (manager side, same codebase) | `Tools_4cc/rigdio` |
| Rigdio fork | The ATF bot's author's fork (v2.2.1, 2026-10-11): upstream v2.2.0 plus a local API and event clips, fed by the SEN:P-AI fork; surveyed 2026-10-13: event clips kept apart from the slots ("Model"), the fixed library path for auto team loading, the stoppage chant and its 15 % default, fuzzy scorer matching (at 90, not its 70) ("Autopilot"); its inbound API is not adopted: its driver joins the suite instead ("Autopilot") | `Tools_Marco/rigdio` |

Playback goes through **libmpv** (`libmpv-2.dll`, pinned to an April 2024 Win7-compatible
build) via the `python-mpv` binding. Loudness analysis shells out to a custom-minified
**ffmpeg.exe** (~1.8MB, built from ffmpeg n4.4.1 with a dedicated MSYS2 build script).

### Relationship to the old codebase

As with the Team compiler, Rigdio defines *what* the new tool must do — the `.4ccm`
format, the selection semantics, the soundboard behavior — not *how*. The codebase is
the product of years of accretion and is not a porting reference:

- **Two near-duplicate `ConditionList` implementations** (`legacy.py` and
  `condition.py`) with subtly different constructors.
- **Conditions evaluated by string-built `eval()`** (`ArithCondition` formats a Python
  expression and evals it).
- **tkinter dialogs opened from inside condition checks** (`TimeCondition` pops a modal
  during evaluation — domain logic directly drives UI).
- **Five polling threads** with `sleep()` loops (song-end checker, chant-end checker,
  title-log timer, fade stepper, loudness-wait spin), plus tkinter `after()` polling
  loops for the load progress and the VA timer.
- **Global mutable module state**: position cache, loudness cache, pending/logged sets,
  `titleThread`/`titleCheck` globals shared across instances.
- **A half-finished YAML successor format** (`toYML()` methods, `InstructionList`,
  "DEPRECATED: PLEASE USE EVENT: IN YOUR .YML") that never shipped — the `.4ccm` parser
  is called "legacy" throughout even though it is the only format.
- **Parser hacks**: RigDJ strips `[...]` brackets on load and `rigparse` re-adds them
  with a special-case loop the comments themselves call "very messy".
- **Dead code that still runs**: the Match tracker's event system (red/yellow/sub/own-goal clips)
  lost its trigger when the tracker integration was removed in v1.11 — clips are still
  parsed and loaded into an `EventController` that nothing in upstream fires; the Rigdio
  fork (v2.2.1) fires it again from a local HTTP API, fed by the SEN:P-AI 1.3.0 rebuild.
- **Two external binaries** pinned to Win7-era builds, plus PyInstaller/UPX packaging
  machinery and a bespoke ffmpeg build script to maintain them.

---

## The `.4ccm` format (`libs/music_export`)

**The `.4ccm` format stays canonical** — full read *and* write support, no successor format.
Existing working exports remain supported. Interoperability is limited by legacy parser bugs:
Studio's restored `time` condition cannot load in affected Rigdio releases, and mixed-case
`opponent` matching may behave differently there. The editor warns about these known exceptions;
unchanged syntax is not a guarantee of identical behavior in buggy legacy readers.

### Grammar

Plain text, one entry per line, `;`-separated fields:

- Lines are whitespace-trimmed. Empty lines and lines starting with `#` are comments. A
  leading UTF-8 byte-order mark is dropped (Notepad writes one; Rigdio, reading in the
  locale's encoding, sees it as part of the first line).
- First significant line: `name;<teamname>` (team name is matched case-insensitively).
  If absent, the filename stem is used and a warning is emitted. A `name` line anywhere
  else is refused as a reserved word, as Rigdio reads only the first significant line
  (`rigparse.py:33-41`) and takes a later one for a slot.
- Then optional flag lines, in any order: `sync;<val>` and `normalize;<val>`, where
  a value of `no`/`off`/`false`/`0` disables and anything else enables, keyword and value
  read case-insensitively as Rigdio does (`rigparse.py:47-60`: `Sync;No` is a flag, not a
  player named Sync). Both default to enabled. Blank and comment lines may sit between them,
  which Rigdio does not allow; lenient on purpose, since it loses nothing.
- Every other significant line is a song entry:
  `<slot>;<filename>;<condition-or-instruction>;<...>`
  - **slot** — `anthem`, `victory`, `goal` (default goalhorn), `chant`, or a player
    name (free text; commas in a name render as line breaks on the button).
  - **filename** — path relative to the `.4ccm`'s folder. If the line has only a slot,
    a default is assumed: `{team} - {Label}.mp3` for reserved slots (Anthem, Victory
    Anthem, Goalhorn, Chant) or `{team} - {player} Goalhorn.mp3` for players.
  - Each further `;`-field is one condition or instruction: the first whitespace-token
    is the type, the rest are arguments. `[bracketed text]` groups a spaced string
    into a single token; `\[`, `\]`, and a leading `\` escape. An empty group `[]` is no
    token (`special []` reads as `special`), and the writer never writes one, nor brackets
    around a single word: Rigdio's group runs from the `[` word to a *later* word ending in `]`
    (`processTokens`, `condition.py:816-837`), so on either it crashes at the line's end or
    swallows the words after it.
- Reserved words (not usable as player names): `name`, `sync`, `normalize`, `anthem`,
  `victory`, `goal`, `chant`. (Rigdio's set also contained the literal string
  `;event`, unreachable since `;` is the field separator — dropped here; `event`
  is an instruction, never a slot, so it needs no reserving.)

### Conditions

All conditions on an entry must hold (AND), checked in the order written and stopping at
the first that does not (Rigdio's `ConditionList.check`, `legacy.py:363-370`): a condition
after a failing one is not evaluated, so it neither unloads its entry nor spends a `once` nor
asks for the goal minute. Entries are checked top-down per slot; the first passing entry plays
(priority order).

| Condition | Arguments | Plays when |
|-----------|-----------|-----------|
| `goals` | op, n | this player's goal count compares true (`=`/`==`, `!=`, `<`, `>`, `<=`, `>=`) |
| `teamgoals` | op, n | the team's total goals compare true |
| `lead` | op, n | goal difference (us − them) compares true |
| `every` | n | player's goal count is divisible by n |
| `opponent` | team names | the opponent is one of the listed teams |
| `match` | type list | match type is one of: Group, Survival, RO16, Quarterfinal, Semifinal, Final, Third-Place, Boss, Consolation; `knockouts`, as the only argument, expands to RO16…Third-Place; beside other types it is refused (Rigdio reads it there as a type no match has) |
| `home` | — | the team is in the home slot |
| `once` | — | the first time the check reaches it; the entry is unloaded the next time (`goals >= 2;once` plays on the second goal) |
| `first` | — | this is the team's first goal (score == 1 after increment) |
| `comeback` | — | after scoring, still level or behind, and the opponent has scored |
| `mostgoals` | [player] | this (or the named) player has the most goals on the team (ties count) |
| `special` | [label] | never auto-plays; marks a victory anthem selectable from the VA dropdown |
| `time` | op, minute | the goal minute compares true; the minute is prompted from the streamer when the check reaches it |
| `not` | condition | negation of the wrapped condition |

Conditions that can never become true again (`once` after firing, `time` with `<`/`<=`/
`==` once the minute has passed) cause the entry to be **unloaded** — removed from the
priority list for the rest of the match. Rigdio modeled this as an `UnloadSong`
exception; the new model returns a three-valued verdict (`True`/`False`/`NeverAgain`).

Two correctness notes against current Rigdio:

- **`time` is a regression there**: the `TimeCondition` class exists (added v1.8) but
  was never registered in the parser's condition table, so a `.4ccm` using `time`
  fails to load with "condition/instruction not recognised". The new parser restores
  it as documented.
- **`opponent` matching is lowercase-only there**: team names are lowercased on load
  but the condition's tokens are compared verbatim, so `opponent Eagles` never
  matches. The new player compares case-insensitively, which keeps all working files
  working and un-breaks mixed-case ones.

### Instructions

Instructions modify playback rather than gate it:

| Instruction | Arguments | Effect |
|-------------|-----------|--------|
| `start` | time (`[[d:]h:]m:s` or seconds) | begin playback at this offset; looping returns to it |
| `speed` | 0.25–4.00 | fixed playback speed for this entry (the parser takes any positive rate; the range is the editor's check, DECISIONS 2026-10-14) |
| `randomise` | — | when all of a player's (non-warcry) entries have it: pick uniformly at random instead of by priority |
| `pause` | `continue`\|`restart` [`every` n] | on pause: keep position, or restart (every nth pause) |
| `end` | `stop` | do not loop; stop and reload at song end (`end loop` is a legacy no-op, silently accepted) |
| `warcry` | — | play this entry to completion first, then fall through to the next non-warcry entry |
| `unrandom` | — | exclude this chant from random chant selection |
| `advance` | — | at song end, play the next valid entry instead of looping |
| `louder` | — | mark for the team's volume-boost slider |
| `event` | `red`\|`yellow`\|`owngoal`\|`sub` | play this clip once (no loop; `start` honoured) when the Match tracker reports the event for this player (see "Autopilot"); inert without the tracker. The entry leaves its slot's goalhorn list: it is never in the priority order, the `randomise` all-entries check or the `goal` fallback, and its conditions are never evaluated (see "Model") |

### File resolution

Song paths resolve against the `.4ccm`'s folder with the same semantics as Rigdio's
`songCheck`:

- Case-insensitive filename matching (Linux support).
- `_normalized` companion files (from the retired Riglevel pre-normalization workflow):
  when normalization will *not* be applied to the team (globally off, or the team opted
  out), a `<stem>_normalized` file of any extension is preferred over the plain file
  (the one with the song's own extension when there are several); when the plain file is
  missing, the companion is used regardless. The extension is free because `songCheck`
  compares stems alone, and real exports rely on it: IZAKAYA's pairs every `X.mp3` with
  an `X_normalized.opus` (17 songs), DNG's one.
- Missing files are collected across the whole export and reported as one list, not
  one-at-a-time.
- A resolved path carries the file's name as it is on disk, not as the export spells it, so
  two spellings of one file are one path (positions are shared per path, "Sync"); a lookup
  reads the folder's listing rather than asking whether the written path exists, which a
  case-insensitive filesystem answers with the written spelling.

Rigdio reads a file, and RigDJ writes one, in the system locale's encoding (Windows-1252 on
the community's machines): a UTF-8 file's accented names read as mojibake there, and a file
holding one of the five bytes Windows-1252 leaves undefined stops the load with
`UnicodeDecodeError`. The new parser reads UTF-8 with a lossy Windows-1252 fallback and warns
instead of crashing. Which encoding the editor writes, for files a Rigdio user still loads, is
an open point of the [music export editor](music_export_editor.md), settled before it is built.

The lookup as built (10.0d; native only, behind `#[cfg(not(target_arch = "wasm32"))]`, since
the wasm build has no filesystem):

```rust
pub struct Resolution {
    pub found: Vec<(String, PathBuf)>,   // each resolved `SongEntry::file`, with the path it resolved to
    pub missing: Vec<String>,            // the files that resolved nowhere, each once, in file order
}

pub fn resolve_song(folder: &Path, file: &str, normalization_applies: bool) -> Option<PathBuf>;
pub fn resolve_export(export: &MusicExport, folder: &Path, normalization_applies: bool) -> Resolution;
```

`resolve_song` joins `file` onto `folder` (a `\` separator is read as `/`, so a path written
on Windows resolves on Linux; a song is always under the export's folder, as in Rigdio, which
concatenates the folder and the name (`rigparse.py:117`): a leading separator is dropped, so
`/x.mp3` is the export's `x.mp3`, and a path naming a drive (`C:/x.mp3`) resolves nowhere; no
export on the machine exercises either: 0 of its 1,808 song lines are anchored or name a
subfolder) and tries two
candidates in the joined path's folder, by its file name: the plain file, and the companion,
a file whose stem is `<stem>_normalized` whatever its extension, the song's own extension
first when there are several. With `normalization_applies` unset the companion comes first,
with it set the plain file. Each candidate is its own lookup in the folder's listing, names
matched case-insensitively (Unicode folded, as `songCheck` lowercases), the exact spelling (or
the song's own extension) preferred and among equals the smallest name, so the filesystem's
listing order decides nothing (the subfolder components are used as written); a path with no
file name (`sub/`) and an unreadable folder find nothing. `resolve_export` looks up every
distinct file once, in the model's order: slot by slot in order of first appearance, file
order within each, then the event clips.

### Model

```rust
pub struct MusicExport {
    pub team: String,
    pub sync: bool,          // shared goalhorn positions (default true)
    pub normalize: bool,     // loudness normalization opt-out flag (default true)
    pub slots: IndexMap<Slot, Vec<SongEntry>>,   // file order preserved; no `event` entries
    pub events: Vec<EventClip>,                  // the `event` entries, file order preserved
}

pub enum Slot { Anthem, Victory, Goal, Chant, Player(String) }

pub struct SongEntry {
    pub file: String,                     // as written (relative path)
    pub conditions: Vec<Condition>,       // typed enum, one variant per table row above
    pub instructions: Vec<Instruction>,   // typed enum
}

pub enum EventKind { Red, Yellow, OwnGoal, Sub }

pub struct EventClip {
    pub player: String,       // the line's slot field (a player name, matched case-insensitively)
    pub kind: EventKind,      // the `event` instruction's argument
    pub entry: SongEntry,     // the line's other fields; conditions kept for the round trip, never evaluated
}
```

- **Event clips live apart from the slots.** The parser puts a line carrying `event` into
  `events`, its `event` instruction lifted into `kind`, not into `slots[Player(name)]`, so
  selection over a slot stays a plain top-down loop with no skip, and the `randomise`
  all-entries check and the `goal` fallback append need no exclusion either. Rigdio's parser
  routes them the same way (its `events` dict beside `players`); RigDJ drops them on load,
  so the editor's round trip keeps what RigDJ loses.
- **One model, one parser, one serializer** — replaces the two divergent
  `ConditionList` implementations. RigDJ's bracket-stripping/re-adding hack disappears
  because tokens survive the round trip typed instead of as strings.
- Evaluation is pure: `Condition::eval(&self, state: &MatchState, side: Side, player: &str)
  -> Verdict`. No `eval()` strings, no dialogs — `MatchState` carries
  `goal_minute: Option<u32>`, and the *view* prompts for the minute when the press's walk
  would reach a `time` condition (the `needs_goal_minute` query, a dry run of the selection),
  before the selection runs: Rigdio prompts at that point of its check, so a press that
  plays before any `time` is reached asks nothing. When a Match tracker
  event supplies the goal clock, `goal_minute` comes from `Clock.display_minute` and the
  view skips the prompt. Otherwise the ordinary view-driven prompt rules apply (see
  "Autopilot").
- `MatchState` (scores, per-player scorer counts, team names, match type, goal minute)
  lives in `music_export` too: plain data + methods, trivially unit-testable, replacing
  `gamestate.py`'s five-mutex thread-safety dance (the GUI thread owns it; workers get
  snapshots).
- The serializer regenerates RigDJ's canonical layout (section comments
  `# team identifier` / `# reserved names` / `# regular players`, plus a fourth,
  `# event clips`, when the export has any; flags written only
  when opted out). Comments in hand-written files are not preserved — same as RigDJ.

The rest of the crate as built (10.0a-d, ahead of Phase 10). The code is the source and
these blocks its copy: a change to one is a change to both, and a shape not here is not
crate API (`CONTRIBUTING.md` "Code style", `pub(crate)` by default). The consumer that
justifies each `pub` item is the music player ("Playback semantics", "Autopilot") or the
[music export editor](music_export_editor.md) ("What the editor does", "New capabilities").

**Conditions and instructions**, one variant per table row (`event` excepted: it lifts its
line into an `EventClip`, above):

```rust
pub enum Comparison { Eq, Ne, Lt, Gt, Le, Ge }   // `==` (`=` reads the same), `!=`, `<`, `>`, `<=`, `>=`
pub enum MatchType { Group, Survival, Ro16, Quarterfinal, Semifinal, Final, ThirdPlace, Boss, Consolation }

pub enum Condition {
    Goals(Comparison, i32),
    TeamGoals(Comparison, i32),
    Lead(Comparison, i32),
    Every(u32),                  // n > 0
    Opponent(Vec<String>),       // case-insensitive, no slashes
    Match(Vec<MatchType>),       // `knockouts` expanded on read
    Home,
    Once,
    First,
    Comeback,
    MostGoals(Option<String>),
    Special(Option<String>),
    Time(Comparison, u32),
    Not(Box<Condition>),
}

pub struct StartOffset { pub millis: u64 }   // `StartOffset::parse(text: &str) -> Option<StartOffset>`;
                                             // `Display` writes the shortest form that reads back (`0:39.6`)
pub enum PauseAction { Continue, Restart }
pub enum EndAction { Stop, Loop }            // `Loop` is the default, kept so the line round-trips

pub enum Instruction {
    Start(StartOffset),
    Speed(f64),                                  // any positive finite rate
    Randomise,
    Pause { action: PauseAction, every: u32 },   // `every` is 1 when not written
    End(EndAction),
    Warcry,
    Unrandom,
    Advance,
    Louder,
}
```

`Comparison`, `MatchType`, `EventKind`, `PauseAction` and `EndAction` have
`keyword(self) -> &'static str`, their spelling in the file (the editor's dropdowns and chips;
`Condition` and `Instruction` have none, the serializer writes them whole);
`Slot::keyword(&self) -> &str` is the reserved word or the player's name,
what a press names as its `player`. `MatchType::ALL` is the nine types in the cup's order and
`MatchType::KNOCKOUTS` the five `knockouts` stands for (the editor's Toggle Knockouts). The
readers (`from_keyword`) are the parser's alone.

**Reading and writing.** The caller owns the file I/O: the parser takes the bytes and the
file's stem (the team name when the file carries no `name` line), the serializer returns the
text. A refusal names its line and its defect as a stable kind, for the tools' `messages.rs`;
a warning is what the reader made up for.

```rust
pub fn parse(bytes: &[u8], file_stem: &str) -> Result<Parsed, ParseError>;
pub fn write(export: &MusicExport) -> String;

pub struct Parsed { pub export: MusicExport, pub warnings: Vec<Warning> }
pub enum Warning { TeamNameMissing, NotUtf8 }
pub struct ParseError { pub line: usize, pub kind: ParseErrorKind }   // Display: `line {line}: {kind}`
pub enum ParseErrorKind {
    EmptyTeamName, ReservedWord(String), EmptySlot, EmptyFile, EmptyField,   // a second `name` is a `ReservedWord`
    UnclosedBracket, UnknownKeyword(String),
    MissingArgument { keyword: String, what: &'static str },
    ExtraArgument { keyword: String, argument: String },
    InvalidArgument { keyword: String, argument: String },
    NotOfInstruction(String), SecondEvent, EventOnReservedSlot(String),
}
```

**Match state**, plain data owned by the GUI thread (the view and the Match tracker write it,
the conditions read it):

```rust
pub enum Side { Home, Away }
impl Side { pub fn opponent(self) -> Side; }

pub struct TeamState {
    pub name: String,                  // as the export carries it (lowercase, no slashes)
    pub score: u32,
    pub scorers: Vec<(String, u32)>,   // in order of first goal; a name is matched case-insensitively, the first spelling kept
}
impl TeamState {
    pub fn new(name: &str) -> TeamState;                 // no goals
    pub fn player_goals(&self, player: &str) -> u32;     // 0 when they have not scored
}

pub struct MatchState {
    pub home: TeamState,
    pub away: TeamState,
    pub match_type: MatchType,
    pub goal_minute: Option<u32>,      // `None` until prompted or supplied, and after each play
}
impl MatchState {
    pub fn new(home: &str, away: &str, match_type: MatchType) -> MatchState;   // 0-0
    pub fn team(&self, side: Side) -> &TeamState;
    pub fn opponent(&self, side: Side) -> &TeamState;
    pub fn score_goal(&mut self, side: Side, player: &str);
    pub fn unscore_goal(&mut self, side: Side, player: &str) -> bool;   // false when there is none to take (Undo)
    pub fn clear_team(&mut self, side: Side);                           // the per-team Reset
}
```

**Evaluation**, pure functions of the state:

```rust
pub enum Verdict { True, False, NeverAgain }   // holds now; not now, maybe later; never again this match (unload)

impl Condition {
    pub fn eval(&self, state: &MatchState, side: Side, player: &str) -> Verdict;
    pub fn needs_goal_minute(&self) -> bool;   // a `time`, under `not` included
}
impl SongEntry {
    pub fn eval(&self, state: &MatchState, side: Side, player: &str) -> Verdict;   // AND in written order: the first verdict that is not `True`
    pub fn needs_goal_minute(&self) -> bool;   // carries a `time` anywhere (the editor's Rigdio warning)
    pub fn is_warcry(&self) -> bool;      // the player's play-to-completion, the editor's Randomise Horns skip list
    pub fn is_randomise(&self) -> bool;   // the editor's Randomise Horns mixed state
}
```

The counts include the goal being scored. `once` is `True` in a pure evaluation, and the
selection, which remembers, makes it `NeverAgain` once spent: it is spent the first time the
check reaches it (every condition before it held), as Rigdio's `OnceCondition.check`
(`condition.py:250-261`) does; `time` without a minute is `False`; `not` of a `NeverAgain` is
`True`, the negation holding from then on, where Rigdio unloads the entry (its `UnloadSong`
passes through `not`): a departure, since the negation does hold from then on, and no file on
the machine writes one; `special` is `False`, never auto-played (DECISIONS 2026-10-14).

**Selection**, one set of lists per team:

```rust
pub struct EntryId(/* private */);   // Copy, Eq: one entry of a team's lists, stable for the match
pub struct Loaded { pub entry: SongEntry, /* id, once_spent: private */ }
impl Loaded { pub fn id(&self) -> EntryId; }
pub struct TeamLists { /* each slot's own entries, shrinking as entries unload; the `goal` slot's once */ }

pub struct Press {
    pub side: Side,
    pub skip: Option<EntryId>, // the entry that just ended (`advance`)
    pub after_warcry: bool,    // a warcry already played for this press: warcries are passed over
}

impl TeamLists {
    pub fn load(export: &MusicExport) -> TeamLists;
    pub fn entries(&self, slot: &Slot) -> Vec<&Loaded>;   // the button's list: the slot's own, then for a player the `goal` slot's
    pub fn needs_goal_minute(&self, slot: &Slot, state: &MatchState, press: Press) -> bool;
    pub fn select(
        &mut self,
        slot: &Slot,
        state: &MatchState,
        press: Press,
        pick: &mut impl FnMut(usize) -> usize,
    ) -> Option<usize>;
}
```

- A button's list is its slot's entries, then the `goal` slot's as the fallback when the slot
  is a player's (reserved slots and `goal` itself get none). The fallback is **one list per
  team**, not a copy per button: Rigdio appends the same entry objects to every player's list
  (`rigparse.py:169-172`), so an entry unloaded or a `once` spent through one button is gone
  for every button, the `goal` button's own included. A fallback entry evaluates under the
  name `goal`, so its `goals` counts the default goalhorn button's presses, and it is not the
  player's for the `randomise` check or pool, which are its own group's (below).
- `select` walks the button's list as "Song selection" says and returns an index into
  `entries(slot)` as it stands after the press's unloads, or `None` for "no song found"; the
  caller plays `entries(slot)[i].entry`. The random pick is the caller's: `pick(n)` returns an
  index below `n` (the scenario tests are deterministic, the player passes its RNG).
- The entry that ended is named by its `EntryId`, not by its index: between a song's start
  and the `advance` press at its end, another button's press can unload an earlier entry of
  the shared fallback, and an index would then name another entry or none. The id is given
  at load and never reused, so it names the same entry for the whole match.
- `needs_goal_minute` is the same walk on the lists as they stand, changing nothing: whether
  it reaches a `time` condition (under `not` too) while `state.goal_minute` is `None`. The view
  asks it, prompts when it says so, sets the minute, then calls `select`; the walk up to the
  first `time` is the same with or without the minute, so the two agree.
- The lists are plain data (`Clone`): the player snapshots them per goal for Undo and rebuilds
  them from the export for Reset, with no reload. A Reset also drops a pending `advance`
  press: its `EntryId` names an entry of the lists it was given, and the rebuilt ones number
  theirs afresh.

---

## Playback semantics

What the soundboard must do, distilled from Rigdio's behavior:

### Song selection (per button press)

1. Pressing a player's goalhorn button **scores a goal** for that player (per-player
   scorer counts + team score; a snapshot is taken for undo), the Standard Goalhorn under
   the name `goal`. Reserved buttons (anthem, VA, chant) don't score. Pressing again pauses
   (buttons are toggles); the pausing press takes nothing back. A press while the button's
   sound fades out is a new press: it scores and selects.
2. The entry to play is chosen from the button's list, top-down: skip the entry that just
   ended (for `advance`), check the entry's conditions in order ("Conditions"), unload it
   on `NeverAgain`, play the first that passes. The default goalhorn (`goal`) list is
   every player's fallback, after their own entries, one list shared by the team's buttons
   ("Model"). Event clips are not in any slot's list (see "Model"), so selection never sees
   them. This is Rigdio's `getSong` (`legacy.py:618-700`), steps 3 to 6 included.
3. **Randomise**: when the walk reaches a non-warcry entry carrying `randomise` (whatever its
   own verdict, once it is not unloaded) and every non-warcry entry of its group carries it
   too, one of that group's non-warcry entries is drawn uniformly instead, conditions
   unread. The group is the player's own entries, or the fallback's for a fallback entry
   (Rigdio's pool is the entries of the reached entry's `pname`). The entry that just ended,
   step 2's skip, counts for the check and is not drawn, where Rigdio may redraw it: a
   departure (DECISIONS 2026-10-18), since a horn that ends and restarts itself is what
   `advance` exists to avoid.
4. **Warcry**: warcry entries play first and to completion, then selection re-runs
   excluding warcries. Before the walk, when every warcry in the button's list (the
   fallback's included, the skipped one excepted) carries `randomise`, one of them is drawn.
5. **`advance` with nothing passing**: when the press follows an `advance` and the walk finds
   nothing, the first non-warcry entry other than the one that ended plays, conditions
   unread; with none, the one that ended plays again (`legacy.py:683-695`).
6. Otherwise, if nothing passes, report "no song found" (never crash); the goal the press
   scored still counts.

### Playback behavior

- Goalhorns and anthems **loop by default**; victory anthems and chants do not.
  `start` makes the loop return to the start offset.
- **A button owns what it plays.** Each button holds one playback state (stopped, playing
  a sound, fading one out), and every sound it starts is held in that state: starting
  another stops the one held, so no sound is ever left playing that its button cannot
  reach. One exception: a press during a fade-out that selects another entry lets the
  fading sound finish its fade beside the new one, as Rigdio does, since a fade ends by
  itself within its time; a press that selects the fading entry again cancels the fade,
  and the sound goes on at full volume from where it is (Rigdio jumps it back to where the
  fade began, an artifact of its position cache). Song end, a warcry's hand-off to the next entry and `advance` are transitions of
  that state taken in the tool's `tick()`, never callbacks of the sound: a warcry hands off
  only while its button is still *playing* it, so a press that turns the button off in
  the same frame the warcry ends, or a warcry that reaches its end during the press's
  fade-out, starts nothing. Rigdio ran the hand-off on its end-checker thread while the
  press paused on the GUI thread; a press landing as the warcry ended left the goalhorn
  playing with no button holding it, and nothing could stop it short of a restart (a
  streamer's report, 2026-10-26).
- What a button shows and does once its song has played to its end (a horn with `end stop`,
  a victory anthem) waits on `docs/QUESTIONS.md` "A button after its song ends".
- **⟲** next to a button stops its sound at once, without a fade, and puts the entry it
  played last back to its start: `start` and `speed` apply again on its next play, its
  synced position is dropped and its `pause ... every n` count restarts; the VA's ⟲ also
  zeroes the VA timer. The score, the button's other entries and its unloaded entries are
  untouched.
- **Fade-out on pause** (per-type configurable: anthem/goalhorn/victory/chant, default
  2s, skipped if the song already ended). No press pauses a chant, so `fade.chant` governs
  the chant timer's fade and Stop Chant Early (Rigdio fades both always; the default, on,
  keeps that). A chant that plays to its end does not fade.
- **Sync**: goalhorn playback positions are cached per absolute file path and restored
  on the next play of the same file — switching between players sharing a file resumes
  where it left off. Excluded for warcries; cleared on reset/reload; opt-out per team
  via the `sync` flag. Event clips are not synced. `pause restart` restarts under sync
  too: Rigdio v2.2.0 restores the cached position over the restart's seek, but the
  exports that use it (12 of the 61 on the maintainer's machine, each on a goalhorn beside
  `end stop`) were written before its position cache, when it restarted.
- **Anthem exclusivity**: playing one side's anthem pauses the other side's (waiting
  out its fade). Rigdio only wired this one way — home pausing away — via a button
  hook; the new player makes it symmetric. Other buttons are independent (multiple
  horns *can* overlap; that is the Chaoshorn's whole gimmick).
- **Victory anthem**: when the VA list has a `special` entry, a dropdown offers "Default"
  and each special by its label, or its file's name when it has none; the button shows
  the choice, which stays until changed, and its press plays that entry with its
  conditions unread ("Default" walks the list, where `special` never passes). No special,
  no dropdown. A timer shows the VA's play position over its length, stopping at its end
  and on pause, zeroed by ⟲ and Reset (Rigdio's counter runs past the end). A team with no
  `victory` entry loads with a warning naming it, with no VA row and no VA timer.
- **Playback speed**: a global 0.25–4.00× slider in steps of 0.25, disabled while any
  button's sound plays. An entry with a `speed` instruction plays at its own speed on every
  play and the slider never applies to it (Rigdio's code; its comment, which said the
  slider took over after the first play, describes code that does not exist). A follow-up
  (a warcry's, an `advance`) plays at the speed its press used; chants and event clips play
  at 1× or their own `speed`.
- **Chaoshorn**: after a confirmation, starts every loaded button on both teams that is
  not playing, and leaves the playing ones playing (Rigdio's toggle paused them). Each entry
  starts once: a button whose selection is an entry another Chaoshorn button already
  started stays raised and holds nothing (in Rigdio those buttons shared one sound; started
  N times in phase it would be louder by about 20·log10 N dB). A second Chaoshorn press is
  ignored until Kill. Whether its presses score goals waits on `docs/QUESTIONS.md`
  "Chaoshorn and the score". **Kill Chaoshorn** stops every button's sound and rewinds its
  last entry as ⟲ does; it starts nothing and scores nothing.

### Chants

- One chant at a time, globally: a chant press while a chant plays does nothing, that
  chant's own button included (Stop Chant Early stops it). Buttons: one per chant, labelled
  with its file's stem less `_normalized`, plus a Random button per team (Random buttons
  also sit on the main view). A chant starts from its `start` offset, or 0, every time; its
  conditions are never evaluated.
- Random selection uses **exponential decay weighting** (weight = `w^times_played`,
  `w` configurable, default 0.3) so repeats become increasingly rare; `unrandom`
  entries are excluded from random selection. One tally per team, shared by the main view's
  Random button and the chants panel's (Rigdio kept one per button and reset the panel's
  at each opening); a team's load or Reset resets it. With `w` at 0, once every chant has
  played the tally starts a new round (Rigdio's Random button then raised an error and did
  nothing until the next load).
- A chant timer (20–60s, default 30, checkbox to disable) fades the chant out when it
  expires; "Stop Chant Early" fades immediately. The timer's checkbox and slider are locked
  while a chant plays.

### Match state

- Score display, match-type selector, **Undo Last Goal** (decrements score/scorer and
  restores the pre-goal playback snapshot: cached positions, first-play flags, warcry
  state, the lists). One level: a second Undo does nothing until the next goal, since an
  older goal's snapshot no longer describes the buttons after later presses. Undo starts
  and stops no sound and does not seek the one playing.
- **Per-team Reset**, after a confirmation: stops that team's music and its chant if one
  plays, seeks everything to the start, clears position caches and VA timer, resets chant
  randomization and score, rebuilds the lists (unloaded and spent entries come back) — in
  place, no file reloading.
- **Loading a team** sets both scores and both scorer tables to zero and clears the goal
  minute: a load is how a streamer starts the next match, as in Rigdio. The loaded slot
  starts fresh; the other side's playback (its lists, unloads, positions) stays as it is.
  The autopilot's own rule for the same pair again ("Autopilot") is separate.
- A Reset or a load drops the Undo record of every side whose score it zeroes (Rigdio kept
  it, and an Undo then drove the score to −1).

### title.log

When enabled, the currently playing song's `♪ artist — title` (from audio metadata,
filename fallback) is written to `title.log` for OBS text sources, cleared after a
configurable duration (or when the song ends). Symphonia exposes the metadata tags
in-process; the 1-second sleep-for-mpv-metadata hack and its global thread flags
disappear. Written for goalhorns, anthems and victory anthems, not for a warcry (its
follow-up is written); whether chants are written waits on `docs/QUESTIONS.md` "title.log
and chants". One title at a time: a song's start replaces the last one, and the file is
cleared when the song is paused. Created empty at startup when the setting is on, and
emptied when the suite closes, so an overlay never keeps a stale title.

---

## Autopilot (Match tracker integration)

When the [Match tracker](match_tracker/README.md) is running, the player subscribes to its
in-process `match_feed` (typed match events, a reconstructed game clock, and explicit
lifecycle events — the feed contract lives in that plan). A feed chip in the center
column shows the detected teams and clock, next to an **Autopilot mode selector**:
**Full** (default while the feed is live), **Co-pilot**, **Assist**, or **Off**. Every
manual control stays live at all times — autopilot adds presses, it never blocks them.
This restores in-process, and greatly extends, the Rigdio↔SEN:P-AI integration that Rigdio
dropped in 2020 and the two forks (Rigdio v2.2.1, SEN:P-AI 1.3.0) restored as HTTP between
two processes; the end goal is a match the streamer doesn't have to touch. Autopilot runs in
the tool's background `tick`, not its view — watching the tracker's stats table doesn't
pause the soundboard.

The player has no inbound control API. The Rigdio fork's local HTTP API (load, anthems,
goal, victory, chant, event) serves rigbot, a driver outside the player that commands what no
memory cue covers (the anthems, the victory anthem); a driver like it joins the suite as a
Studio tool and calls the player in-process, as the tracker's feed does, rather than over a
port (`core/development_plan.md` "Unscheduled tools").

Snapshot-bearing messages (`Snapshot`, `MatchLive`, `MatchResumed`, `ScoreReconciled`)
are state reconciliation only — the player never synthesizes one-shot automation from
differences between snapshots; only explicit `Goal`, `OwnGoal`, `Card`, and `SubOn`
events with eligible live provenance (`LiveObserved` or `LiveDerived`) trigger automation.
Clock updates arrive on the feed's separate coalesced `ClockWatch` lane (the envelope
and lane contract live in the Match tracker plan).

- **Auto team loading**: on `TeamsDetected` or any snapshot-bearing reconciliation, each
  detected side is resolved independently. An empty slot is loaded from the configured
  **music library folder**: first `<library>/<name>/<name>.4ccm`, `<name>` being the in-game
  name with its surrounding slashes stripped, matched case-insensitively (the fixed path
  the forks' tooling already produces); when that file is absent, a scan of the library's
  exports by their `name` field (the filename stem when absent), case-insensitive: every
  `.4ccm` in the library's subfolders, and in a `Music/` folder inside them, so a library that
  is a folder of Complete exports (`plans/aesthetics_export/player_folders.md` "Complete
  export") or the organizer's pack of their `Music/` folders works as it is. Detecting
  the same pair again (a re-attach, a re-found table) is a no-op that keeps the score. If a
  slot was auto-loaded for an earlier feed team and the
  detected team identity changes, reset and replace that slot. Never overwrite a manually
  loaded slot; instead retain it and show a team-mismatch warning. Provisional team
  corrections follow the same ownership rule. Lookup misses are logged, never fatal. The
  match-type selector stays manual — matches run in Exhibition mode, which carries no
  cup-stage info; the planned migration to Cup mode may make this auto-populatable (see
  the Match tracker plan's open items).
- **Score from the game**: feed score authority begins with `MatchLive`, is reacquired
  by `MatchResumed`, and is taken directly from the snapshot's `score_authority` field
  during reconciliation — the player never infers it from clock presence. Autopilot mode
  controls playback and highlighting only; selecting `Off` suppresses tracker-triggered
  playback and highlights but does not transfer score authority — manual score accounting
  resumes only after `MatchSuspended { reason: UserStopped }`, `MatchFinalized`,
  `MatchAbandoned`, or `FeedReleased`. While the feed is score authority, no manual
  soundboard action — Undo, per-team Reset, or team load/reload — may alter authoritative
  score, scorer totals, discipline, or participation state; those actions affect playback
  state, caches, and loaded audio only. Once authority is released, they resume their
  ordinary manual match-state behavior.
  A `Goal` event with `OpenPlay`, `Penalty`, or regulation-phase `Unknown` kind
  immediately increments the side's regulation score and the scorer's count (scorer
  matched to a player slot as "Scorer matching" below says; no matching slot → the standard
  goalhorn, the goal still counts) — unless `resolves_unclassified` is `Some(id)`, in which case the
  player removes that exact uncertain record before applying the classified score and
  scorer update, rather than adding a second goal; the goalhorn then plays through the
  normal selection logic **after a settable announcement delay** (default 8s) — the window
  where the caster calls the goal ("Goal! Player X, it's 1–2!") before the horn comes in,
  matching how streamers time it manually. The announcement deadline is measured from the
  goal's original monotonic observation timestamp, not from message receipt. A pending goal
  restored without an in-process monotonic anchor is reconciliation-only and can never
  trigger one-shot automation. `ScoreReconciled` replaces the corresponding canonical
  snapshot state without inferring one-shot automation from the change. A `Shootout` goal
  updates only the shootout score and does **not** affect regulation score, scorer counts,
  or the `goals`/`teamgoals`/`first`/`mostgoals` condition state — whether a shootout
  success plays the standard horn is a separate setting (default off, since shootout horns
  are not a stream convention). `GoalAmended` updates the stored assist and kind for
  presentation but never re-triggers scoring or automation.
  Manual goalhorn presses become pure soundboard — they play music without scoring — so
  the two sources can't double-count, and a manual press during the delay cancels the
  pending auto-play. **Undo Last Goal** restores only the pre-goal playback snapshot; it
  does not decrement authoritative score or scorer totals while the feed is score
  authority. Tracker-side score correction is not supported in the first implementation.
- **Scorer matching**: the feed's scorer name is matched to the side's player slots
  case-insensitively first; failing that, among the slots whose name has at least 4
  characters, to the one with the highest partial ratio, if the scorer's name has at least 4
  characters too, the ratio is at least 90 and no other slot ties it (characters are counted
  after trimming, as Unicode scalar values). The partial ratio is `thefuzz`'s
  `partial_ratio`: both names lowercased, the shorter scored by indel similarity
  (`200 × LCS / (len a + len b)`) against each substring of the longer of its own length and
  each shorter prefix and suffix of it, both ways round when the lengths are equal, the best
  score kept and rounded to a whole number (a few lines in the crate, no dependency; a
  hand-written copy matched `thefuzz` 0.22 on 50,000 random pairs). No such slot, or a tie →
  the standard goalhorn, and the scorer's name and the side's slots are logged so the manager
  can rename the slot. A slot name is the manager's to write and is often shorter or spelled
  otherwise than the in-game name (the Rigdio fork's author matched at 70); 90, not 70,
  because managers are expected to bring their slot names close to the in-game ones, and a
  near-miss below it is more likely another player than a respelling. An exact match always
  wins, so a slot named as in the game is never outscored by a fuzzy one. A name contained
  in a longer one scores 100 whichever is the slot, so a name under 4 characters ("I", "NO",
  "sex", all real in-game names) matches exactly or not at all: contained, it would claim
  any slot spelling it; from 4 characters on a contained name ("Mark" in "Markus") still
  scores 100, and the logged names are how a manager finds it. The Studio beta's logs are
  where 90 and 4 are checked. Measured 2026-10-14
  (`.tmp/music/scorer_census/`: 65 slots of the 7 teams on the teams list among the
  machine's `.4ccm` files, against every name and shirt name those teams carry in 144 saves):
  34 slots equal a name exactly, 13 more score 100 by containment (right, "Billy" in "Billy
  Herrington"; wrong, the in-game "sex" in the slot "Sexy Hagcat", or shirt names "NO" and
  "I" in longer slots), none scores 81 to 99, and the other 18 score 80 or less, most of them
  slots of a file copied from another team's. So between 81 and 100 the threshold changes
  nothing on this data; a short name contained in a longer one is what fuzzy matching gets
  wrong, which the 4-character floor answers ("Billy", "Stella" and "Get Out" still match).
- **Goalhorn stop window**: a goal freezes the game clock through the celebration and
  the replay; the clock resumes at the kickoff, a few seconds after the streamer
  dismisses the replay. On the first `ClockStarted` after a goal, autopilot **fades
  the goalhorn out** (configurable, on by default) — the same rhythm as the manual
  habit of stopping the horn right after closing the replay. Replay state itself is
  not visible in the known memory tables, so the kickoff clock-restart is the proxy
  (see "Reading the game" in the Match tracker plan). If the kickoff somehow arrives
  while the announcement delay is still running, the pending auto-play is dropped
  rather than starting a horn into live play.
- **Co-pilot mode**: the scorer's goalhorn button **highlights with a pulsing
  accent**, and the streamer presses it — the highlighted press plays without
  scoring like any manual press. Everything without a press of its own stays
  automatic: team loading, feed-driven scoring, goal minutes, and the **event
  clips**, which fire automatically because they have no buttons at all (brief
  one-shots — the one automation even Rigdio v1.9 ran unattended).
- **Assist mode**: nothing plays by itself, ever. The goalhorn highlight stays, but
  event clips go silent along with everything else — they have no buttons to press,
  so Assist is for streamers who want even the one-shots under manual timing (use
  Co-pilot to keep them).
- **Goal minutes** for `time` conditions come from the feed clock's `display_minute`
  (consumer-ready conventional minute). When a Match tracker event supplies the goal
  clock, `goal_minute` is assigned directly from `Clock.display_minute`; the view skips
  the prompt when that value is `Some`. Otherwise the ordinary view-driven prompt rules
  apply.
- **Own goals**: the benefiting side scores and its standard goalhorn plays (after
  the same announcement delay); if the conceding player has an `event owngoal` clip,
  it plays as well. A nonempty `resolves_unclassified` removes that exact uncertain record
  before scoring, just as for `Goal`; it must not count the same delta twice.
- **Event clips revived**: `Card` and `SubOn` events trigger the matching player's
  `event red|yellow|sub` clips in the export of the side the event names (the other side's
  export is never checked, where the Rigdio fork checks both teams' exports for the name,
  `webserver.py:218-220`: the feed names the side, so a same-named player on the other team
  never fires) — one clip per eligible event, picked uniformly at random when
  the player has several of that type, deduplicated by the feed's
  `(match_id, sequence)`, not by minute (distinct events can share a minute). A `SecondYellow`
  dismissal fires the `event red` clip (the same clip a straight red uses); a single `Yellow` fires `event yellow`.
  A clip plays over everything and stops nothing, in full every time it fires, with no
  button; the master volume applies to it as to every sound (Rigdio's clips took the
  normalization gain and boost but not the master).
  The `event` instruction stops being dead weight.
- **Stoppage chants** (Full mode, 15 % by default): on `ClockStopped` past the first minute
  (`display_minute > 1`) for a reason other than `Goal`, `OwnGoal` or `HalfEnd`, Full mode
  draws with probability `autopilot_stoppage_chant_percent` and, on a hit, plays a random
  chant of either side as a Random button press (the chant timer and the one-chant-at-a-time
  rule apply; no draw while a chant plays). Co-pilot and Assist leave chants to the
  streamer: the Random button is a press of its own. The default is 15 %, as the SEN:P-AI
  1.3.0 rebuild ships it: Full mode is for an unattended match, where nobody presses Random;
  0 keeps chants manual.
- **Anthems start manually**: pre-match anthem autoplay is not planned, and there is no autoplay
  setting or feed trigger. The existing kickoff fade-out may still stop a manually started anthem;
  stopping playback does not imply automatic starting.
- **Victory anthems stay manual** (deliberate omission): auto-firing a VA needs a
  reliable end-of-match signal, and the SEN:P-AI 1.3.0 rebuild found the same independently
  (anthems and the victory anthem have no cue in memory, so it leaves them to the
  streamer). `MatchFinalized` is a bookkeeping signal, not a
  game-state observation — by the time the stats table disappears the moment has
  passed, and finalization may arrive well after the fact (or on a mid-match-attached
  draft with incomplete history). If a "returned to team selection menu" game-state
  flag is ever located in memory (the same open item as replay state, see "Reading the
  game" in the Match tracker plan), it joins the feed as a dedicated event and VA
  automation can be revisited.
- **Suspension and feed loss**: `MatchSuspended` (source/read loss) warns the streamer
  and **pauses** automation — no goalhorns fire, but loaded teams, score, and pending
  one-shot metadata are retained for deduplication; nothing that is playing stops. A
  pending horn does not fire while suspended; on `MatchResumed` it proceeds only if its
  deadline has not expired and the reconciled snapshot reports `ClockMotion::Stopped`
  with the same `stop_epoch` as the `ClockStopped` that opened the post-goal window,
  otherwise it is canceled. `MatchSuspended` with reason `UserStopped`,
  `MatchFinalized`, `MatchAbandoned`, and `FeedReleased` revert the player to fully
  manual behavior and cancel all pending tracker-triggered one-shot automation (the
  streamer deliberately stopped, finalized, discarded, or the draft was detached).
  `MatchFinalized` releases feed score authority but neither stops currently playing
  audio nor triggers victory-anthem automation.

---

## Loudness normalization

Rigdio's flagship v2.x feature, kept and upgraded. Old implementation: ffmpeg
`volumedetect` subprocess per file (whole-track RMS mean + peak), a hand-rolled PCM
decode + windowed RMS pass for chants, gain applied as an mpv `af` filter string with
`alimiter`, all cached in module-global dicts.

New implementation, all in-process:

- **Measurement**: the `ebur128` crate (Rust implementation of EBU R128 / BS.1770).
  Integrated loudness (LUFS) replaces the RMS mean; true peak (dBTP) replaces
  `max_volume`. Silent or too-short inputs may have no finite loudness result; do not
  turn that into infinite gain. Leave normalization gain at 0 dB and report the unavailable measurement.
- **Chant loud-part analysis**: R128 momentary loudness (400ms windows) is produced by
  the same measurement pass; the chant reference is the mean of the loudest N% of
  windows (N = `chant_loud_part_percent`, default 20) — same idea as Rigdio's
  1-second-window RMS ranking, without the second ffmpeg decode.
- **Gain**: `target − reference`, target configurable (default −14 LUFS, the streaming
  standard and Rigdio's numeric default). A **limiter engages when
  `true_peak + gain` would exceed the output ceiling** (Rigdio: `alimiter=limit=0.95`; new engine: the
  limiter DSP stage in `audio_engine`). Output protection also accounts for the final mixed
  signal after master/boost/chant gains: individually safe tracks can clip when they overlap.
  Normalization opt-out does not disable that protection.
- **Volume model** — one formula instead of scattered slider/filter interactions.
  A track's effective gain in dB:

  ```
  gain = master
       + chant_offset      (chants only; the Chants Volume slider)
       + boost             (louder-marked tracks only; the Volume Boost slider, −5..+15 dB)
       + normalize_gain    (target − reference, or 0 if the team opted out)
  ```

  Sliders are dB-native (master ±20 dB perceptual scale with colored knobs, mute at
  the bottom). The mpv cubic-volume conversion (`100·10^(dB/60)`) disappears — the
  engine takes dB directly. With normalization off the formula degenerates as in
  Rigdio: no master slider exists, the Chants Volume slider sets chant volume
  directly, and the per-goalhorn sliders control the rest.
- **Per-team control**: the `.4ccm` `normalize` flag opts a pre-normalized export out
  (baseline 0 dB); the streamer's per-team **Normalize: Yes/No** toggle overrides it
  live, re-applying gain to currently playing tracks and triggering analysis or the
  `_normalized`-file swap (reload) as needed — same observable behavior as Rigdio
  v2.2.0 run from source or as the Rigdio fork's build: the packaged v2.2.0 exe never found
  its ffmpeg and skipped normalization, so parity checks never run against it. Two
  departures: the swap never touches the score, the scorers or the lists (Rigdio's reload
  ran a team load, which zeroed both scores mid-match), and a playing sound whose file the
  swap replaces stops. The toggle shows only with normalization on, disabled until a team
  loads, reading "Normalize: Yes" or a bold "Normalize: No".
- **Background analysis**: on team load, all files are analyzed on a worker pool in
  priority order (anthems → goalhorns → chants → victory anthems, matching the order
  the streamer will need them). Playing a not-yet-analyzed track **jumps it to the
  front of the queue** — or gets a dedicated worker spawned for it — so the wait is
  bounded by that one file's analysis, never by the queue (Rigdio just blocked until
  the pool got around to it).
- **Persistent cache** (new): measurements are cached to disk by file identity (path, size, mtime)
  and measurement-algorithm version. Include settings affecting a stored derived value, such as
  chant loud-part percentage, or recompute it from cached underlying measurements. A CLI command
  can pre-warm the cache before a stream.
- The `louder` instruction, the Volume Boost slider and the blinking boost label carry
  over: the slider is shown, and `louder` acts, only with normalization on and when the team
  has louder-marked tracks, chants included; it moves in whole dB from −5 to +15, starts at
  +5 at each team load (a Reset keeps it), and applies live to a boosted sound playing. A
  team opted out of normalization still gets the boost over its 0 dB. The label blinks while
  any boosted sound of the team plays (Rigdio stopped the blink at any pause of the team's
  buttons).

ffmpeg.exe, its MSYS2 build script, and the 30/60-second subprocess timeouts are all
retired.

---

## Audio engine (`libs/audio_engine`)

Pure-Rust playback replaces libmpv. Requirements, mapped from what Rigdio actually
uses of mpv:

| Requirement | mpv feature used | Replacement |
|-------------|------------------|-------------|
| Decode mp3/ogg/opus/flac/m4a/wav | mpv internal ffmpeg | `symphonia` (mp3, vorbis, flac, aac/isomp4, wav) + Opus (below) |
| Play/pause/seek | `pause`, `time_pos` | `kira` sound handles |
| Looping | `loop_file = inf` | `kira` loop regions (incl. loop-to-`start`-offset) |
| Volume in dB | cubic `volume` + `af=volume=XdB` | gain parameter on the track, dB-native |
| Limiter | `af=...alimiter=limit=0.95` | custom `kira` effect (soft-knee lookahead limiter, ~50 lines) |
| Fade-out | manual 100-step volume thread | `kira` tweens (single call, sample-accurate) |
| Playback speed 0.25–4× | `speed` | `kira` playback rate |
| End-of-song detection | `eof_reached` polling threads | sound state queried in the egui update loop (frame-driven, no threads) |
| Metadata (title/artist) | `metadata` property | symphonia metadata reader |
| Duration/position | `duration`, `time_pos` | track position/duration accessors |

- **`kira`** is the recommended mixer (game-audio oriented: tweens, loop regions,
  playback rate, per-sound effects, cpal output, WASM support — the suite's browser
  build keeps working). `rodio` is the simpler fallback if kira's abstractions fight
  the requirements; the `audio_engine` API insulates the tools from this choice.
- **Opus**: symphonia's own Opus decoder is still work-in-progress, so Opus decode
  goes through a pure-Rust decoder crate (e.g. `opus-decoder`, RFC 8251-conformant)
  registered into symphonia's codec registry behind its OGG demuxer. If the pure-Rust
  decoders prove immature at implementation time, the escape hatch is a statically
  linked `libopus` binding (`audiopus`) — still a single binary, no DLL. This is the
  one open verification item; everything else in the stack is production-proven.
- One playback difference to accept or fix: mpv's speed control is pitch-corrected
  (scaletempo) by default, while a mixer playback-rate change is varispeed (pitch
  shifts with speed). Default to varispeed initially — the speed slider is a meme
  feature — and add a time-stretch stage (e.g. `signalsmith-stretch`) only if users
  ask for pitch-corrected speed.
- The loudness analyzer (`ebur128` + the decode loop) lives in this crate too, since
  it shares the symphonia decode path with playback. The R128 measurement, chant
  momentary-loudness windows, and the limiter DSP are large floating-point reduction
  loops over sample buffers — the textbook workload for Rust 1.98's algebraic FP
  methods (`algebraic_add`, etc.), which let the compiler reassociate and vectorize
  FP ops like `-ffast-math` but per-operation and opt-in. Use them in these loops:
  loudness values are compared against a tolerance, not byte-compared, so the
  methods' non-determinism is acceptable here (unlike `model_convert`, whose
  byte-reproducible output contract rules them out).
- The engine owns the audio thread; the tools drive it from the GUI update loop.
  **Zero polling threads**: fades are tweens, song-end is state queried per frame,
  timers (chant, VA, title.log) are deadlines checked per frame. The per-frame work
  runs in the tool's `tick()` — which the shell calls for every tool regardless of
  whose view is on screen, with `ToolContext::request_repaint_after` (`plans/core/gui.md`
  "Event wiring") scheduling wakeups while the
  app is idle — so playback, timers, and autopilot keep working while the streamer
  has another tool (typically the Match tracker) in view.

---

## GUI view

The tool view is not a pipeline tool — no progress grid or log panel. It is the suite's one
special-size tool (`core/gui.md` "Shell layout", "Window size classes"): a streamer places it
beside the PES window, so it declares a compact window size of its own, which switching to it
restores and switching away gives back to the main tools' size; the size itself is set with
the view's draft. It reproduces Rigdio's proven three-column soundboard inside the suite shell:

```
┌───────────────────────────┬─────────────────────┬───────────────────────────┐
│ [Load Home Team] [Reset]  │   /aaa/ vs. /bbb/   │ [Load Away Team] [Reset]  │
│ [Normalize: Yes]          │      2  -  1        │ [Normalize: No]           │
│ Volume Boost [──●──] +5dB │  Match: [Group ▾]   │                           │
│                           │                     │                           │
│ VA 0:42 / 3:10            │ [Chaoshorn]         │ VA 0:00 / 0:00            │
│ [Anthem]             [⟲]  │ [Kill Chaoshorn]    │ [Anthem]             [⟲]  │
│ [Victory Anthem ▾]   [⟲]  │ Speed    [──●──] 1x │ [Victory Anthem ▾]   [⟲]  │
│ ── Goalhorns ──           │ Master   [──●──] +0 │ ── Goalhorns ──           │
│ [Standard Goalhorn]  [⟲]  │ Chants   [──●──] +0 │ [Standard Goalhorn]  [⟲]  │
│ [Player One]         [⟲]  │ [Undo Last Goal]    │ [Player A]           [⟲]  │
│ [Player Two]         [⟲]  │ [Random] [Random]   │ [Player B]           [⟲]  │
│  ...                      │ [Stop Chant Early]  │  ...                      │
│                           │ [Manual Chants]     │                           │
└───────────────────────────┴─────────────────────┴───────────────────────────┘
```

- Team columns scroll independently (long rosters). Playing buttons render "sunken"
  (toggled style); the boost label blinks while a boosted track plays.
- An export with no `anthem` entry, or players but no `goal` entry, loads with a warning
  naming the slot, and that button is absent (the players' buttons then have no fallback).
  Rigdio refuses such a team with a `KeyError` (`songgui.py:353` reads `players["anthem"]`,
  `rigparse.py:172` `players['goal']`): a crash, not a rule, and the rest of the team plays.
- The **Volume Boost slider sits directly below its team's Normalize toggle** (shown
  only when the team has louder-marked tracks). Rigdio's dynamic tkinter grid made it
  look like it lived below the victory anthem row; the intended position is the top of
  the team column, and the fixed layout pins it there.
- **Manual Chants** swaps the center panel's contents for the chants panel (timer
  checkbox + slider, per-chant buttons for both teams, Stop Chant Early, the Chants Volume
  slider, the same value as the center column's, and a back button) rather than adding
  another panel — streamer monitor space is often limited. This replaces the separate
  tkinter Toplevel and its window-lifecycle bookkeeping. Opening the panel stops nothing
  (Rigdio's rebuild of the chant buttons faded out the chant playing).
- Loading runs on a background thread with a progress bar (files analyzed / loaded),
  exactly like the current loading window, but driven by channel events like every
  other tool in the suite. The board stays live meanwhile, except the slot being loaded
  (Rigdio's loading window took every input). A song file that is missing refuses the
  team's load, listing every missing file once, and a failed load leaves the slot as it
  was: a team that loads plays as its author heard it. A missing event clip is a warning
  naming it, the clip left out (Rigdio loads such an export and never fires its clips).
- The `time` condition's goal-minute prompt is a small modal opened by the view when
  a scored goal needs it (skipped while the Match tracker feed supplies the minute).
- When the Match tracker feed is live, the center column gains a feed chip (detected
  teams + game clock) and the Autopilot toggle (see "Autopilot").
- Individual volume sliders (the pre-normalization UI, 0–200) survive only for the
  normalization-off configuration, matching Rigdio: one on every team button, anthem and
  VA included, on the row below it, each with a 🔊 toggle that shows or hides it;
  `show_goalhorn_volume_default` shows or hides all of a team's at load. A move applies live
  and to the button's later plays. Chants have none (the Chants Volume slider).
- Dark/light theme comes from the suite; Rigdio's own palette system and dark-mode
  config disappear. **The functional button colors are preserved**, though — the
  home/away team tints, and the distinct reset/load (green), stop (yellow), kill
  (blue), chaoshorn (red), and normalize (purple) accents are part of how streamers
  navigate the board at a glance. The exact shades may be adjusted to sit well on the
  suite's dark and light themes.

### Settings (tool section)

Rigdio's `config.yml` keys map to the tool's settings section in the suite settings
menu (same semantics, same defaults), with three of Rigdio's built-in values made settings
(`target_lufs`, `fade`, the default match type: Rigdio wrote them outside the file's
`config` mapping, so no streamer could change them; their defaults are those values). A
change applies at once, except `normalize_volume` and the two sort orders, which reshape the
board and apply at the next team load; nothing needs a restart (Rigdio's settings all did, a
relaunch that loses the match). Rigdio's first-run offer to open its settings is dropped.

| Setting | Default | Notes |
|---------|---------|-------|
| `normalize_volume` | on | master switch; hides per-horn sliders when on |
| `target_lufs` | −14.0 | normalization target |
| `chant_loud_part_percent` | 20 | loud-part window share for chants |
| `show_goalhorn_volume_default` | on | only when normalization is off |
| `alphabetical_sort_goalhorns` / `_chants` | off | |
| `chant_timer_enabled_default` | on | |
| `chant_random_decay_weight` | 0.3 | 0 = never repeat, 1 = uniform |
| `write_song_title_log` | 0 | 0 off, N seconds, −1 whole song |
| `fade` (per type + duration) | all on, 2s | |
| default match type | Group | |
| `music_library_folder` | — | autopilot team lookup (see "Autopilot") |
| `autopilot_mode` | full | `full` / `co-pilot` / `assist` (highlight only) / `off` |
| `autopilot_horn_delay` | 8s | announcement window between a detected goal and the horn |
| `autopilot_horn_stop` | on | fade the goalhorn at the post-goal kickoff |
| `autopilot_shootout_horn` | off | allow the standard horn for live shootout successes; never changes regulation score, scorer counts, or regulation condition state |
| `autopilot_stoppage_chant_percent` | 15 | chance (0–100) of a random chant at a stoppage that is not a goal, an own goal or a half end; 0 keeps chants manual (see "Autopilot") |

`write_to_log` disappears (the suite owns logging), as do the palette tables.

### CLI

```
studio music-player analyze ./team.4ccm     # pre-warm the loudness cache for a stream
```

Parsing/validation CLI lives with the [Music export editor](music_export_editor.md)
(`check`), which is the tool aimed at people working *on* the files.

---

## Improvements over Rigdio (summary)

1. **Single binary** — libmpv-2.dll, ffmpeg.exe, the ffmpeg build script, PyInstaller
   spec files, and UPX compression all disappear.
2. **One typed condition model** with pure evaluation — no duplicate class hierarchies,
   no `eval()`, no UI calls inside domain logic, exceptions replaced by verdicts.
3. **Event-driven engine** — the polling threads and `after()` loops become tweens,
   callbacks, and frame-driven state checks; no global thread flags.
4. **R128 loudness in-process** — better measurement (integrated LUFS + true peak) at
   zero subprocess cost, with a persistent cross-session cache and CLI pre-warming.
5. **dB-native volume pipeline** — one composition formula; no mpv cubic conversion,
   no filter-string reapplication races.
6. **Undo, reset, and team-slot swaps as plain state snapshots** on model data.
7. **Suite integration** — settings section, theme, logging, and (via WASM) the
   browser build come from the platform.
8. **Autopilot** — with the Match tracker running, team loading, scoring, goalhorns,
   goal minutes, and event clips are driven by the game itself (manual override
   always available); upstream Rigdio at its most automated only had event clips, and
   the Rigdio fork adds team loading, the scorer's horn and stoppage chants over HTTP,
   without the announcement delay, the kickoff fade or a single score authority.

## Dropped features and compatibility notes

- **Match tracker event clips** (red/yellow/sub/own-goal): dead code in upstream Rigdio
  since v1.11 removed the tracker listener — clips load into a controller nothing triggers
  (the Rigdio fork fires them again from its local API). Not
  dropped here after all: the [Match tracker](match_tracker/README.md) revives them (see
  "Autopilot"). Without the tracker running they stay dormant, exactly as in upstream
  Rigdio, so old files behave identically either way.
- **The YAML format remnants** (`toYML`, `InstructionList`) are not carried over.
- **`end loop`** stays a silently-accepted no-op (goalhorns loop by default).
- **`_normalized` files** keep their resolution semantics for old exports, but the
  workflow that produced them (Riglevel) stays retired.
- Legacy quirks kept: team names are lowercased on load, `opponent` names are written
  without slashes, and the comparison is case-insensitive (see the correctness notes
  under Conditions — Rigdio's own comparison only matched lowercase-written names).
- **Where Studio reads a file as Rigdio does, and where it departs.** A member's `.4ccm` was
  written and tested against Rigdio, so wherever Rigdio's reading decides which song plays,
  Studio's matches it (the walk in "Song selection", the condition order, `once`, the shared
  fallback, the first-line `name`, the case of the flags, a song path kept under the export).
  Where Rigdio crashes, drops what the author typed, or loads a line whose meaning is a typo,
  Studio refuses the line at load, naming it, rather than guess: so a file that loads in
  Studio plays as its author heard it in Rigdio, and one that does not says why. The
  departures of that kind: an extra or malformed argument (`pause restart each 2`,
  `goals >= 2.5`, `every 0` or `-2`, a bare `opponent` or `match`) and an unknown `match`
  word, which Rigdio ignores or never matches; `event` twice on a line, on a reserved slot, or
  under `not`; an empty team name (`name;`) or slot (`;a.mp3`); a `start` time in a form
  Rigdio's reader half-parses; and `not` keeping a bracketed group whole (Rigdio's `not`
  keeps its first word: `not mostgoals [Get Out]` tests "Get"). Kept on purpose besides:
  scorer names and `opponent` matched case-insensitively; a file with no `name` line keeps
  the lowercased stem (Rigdio keeps its case, which nothing compares case-sensitively); a
  song in a subfolder looked up case-insensitively, its `_normalized` companion with it
  (Rigdio's `songCheck` lists the export's own folder only, `rigparse.py:176-199`; no export
  on the machine has a subfolder); a warcry plays again on the button's next press even when
  the follow-up after it found nothing (Rigdio then skips that button's warcries for the
  rest of the match, a latch no author relies on); and an `advance` press whose ended song
  was unloaded meanwhile (its `once` spent, or another button's press unloading the shared
  `goal` entry) plays nothing when nothing else is left, where Rigdio replays the song that
  ended. The player's controls get the same audit when its
  Acceptance section is written (DECISIONS 2026-10-21).

## Testing

- **Round-trip**: parse → serialize → parse on the community's `.4ccm` library;
  semantic equality (model-level), plus golden-file stability for editor-written files.
- **Selection-logic scenario tests**: scripted match timelines (goals, undo, match
  types, warcries, randomise groups, once/time unloads, advance chains) asserting
  which entry plays — pure functions over `MatchState`, no audio needed.
- **Loudness**: compare integrated/momentary loudness and true peak against ffmpeg `ebur128`
  with true-peak measurement enabled on the same decoded samples, within a documented tolerance.
  `volumedetect` measures RMS/sample peak, not LUFS/true peak, so its old values are behavior-change
  references rather than numerical correctness oracles. Test limiter behavior on boosted and
  overlapping tracks as well as single tracks.
- **Engine smoke tests**: decode every supported format (including Opus), loop with
  start offset, fade-out, rate change, EOF detection.
- **Button state**: a warcry's end and a press turning its button off in the same frame,
  in both orders, and a warcry ending during the press's fade-out: nothing plays after,
  and the engine holds no sound the button does not (the "A button owns what it plays"
  rule; driven through the state machine with a fake engine, no audio needed).
- **Autopilot**: scripted `match_feed` event sequences (goals, own goals, cards,
  unknown scorers, a scorer matched fuzzily at 90 and missed at 89, two slots tied, a
  3-character name contained in a slot left unmatched and a 4-character one matched, feed
  loss) asserting scoring, selection, the no-double-counting
   rule, the announcement-delay window (manual press cancels the pending horn; early
   kickoff drops it), the horn fade at the post-goal clock restart, and highlight
   behavior in Co-pilot/Assist modes — pure logic, no game or audio needed.
- **Manual streamer testing** during a real or simulated stream before release.

## Key decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Naming | `music_player` crate; label "Music player (Rigdio)"; existing Rigdio icon in the icon rail | Suite naming convention for the crate/id; the community name and icon stay user-visible |
| `.4ccm` | Canonical, read + write, unchanged | Interop with legacy Rigdio during transition; no format migration for managers |
| Format + model crate | `libs/music_export` | Shared with the editor; one parser/serializer/evaluator |
| Audio backend | Pure Rust: `kira` + `symphonia` + Opus decoder crate + `ebur128` | Single binary, no Win7-pinned DLLs, in-process loudness, WASM-compatible |
| Opus | Pure-Rust decoder registered into symphonia; static libopus as escape hatch | symphonia's own Opus support is unfinished; either path keeps the single binary |
| Loudness | EBU R128 (integrated LUFS + true peak + momentary windows) | Correct measurement; chant loud-part maps onto momentary windows naturally |
| Threading | Engine owns the audio thread; GUI update loop drives all timers/state | Eliminates all polling threads/loops and their shared-state bugs |
| Speed slider | Varispeed initially; time-stretch only on demand | Behavior deviation from mpv's pitch-corrected default, accepted for a meme feature |
| Match tracker events | Revived: `event` clips fire from the Match tracker feed; dormant without it | The tracker returns the integration Rigdio lost in v1.11, in-process rather than over the forks' HTTP |
| Autopilot | Feed-driven scoring/goalhorns/team loading with manual override; manual presses don't score while the feed is live; Full, Co-pilot (highlights + automatic buttonless clips), Assist (highlight-only), or Off (suppresses tracker-triggered playback/highlights but does not transfer score authority) modes; shootout goals excluded from regulation scoring; `SecondYellow` fires `event red`; suspension pauses automation, resumption cancels stale pending horns | One source of truth for the score prevents double counting; end goal is an unattended match, with Co-pilot/Assist/Off as trust-building middle steps |
| Goalhorn window | Horn plays after a settable announcement delay (default 8s); fades on the first clock restart after the goal | Streamers let the caster call the goal before the horn; replay state isn't in the known memory tables, so the kickoff clock-restart is the closest proxy for "replay dismissed" |
| Condition prompts | View-driven (`needs_goal_minute` query + modal) | Domain logic never opens dialogs |
